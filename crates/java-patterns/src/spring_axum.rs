/// Phase 4: Spring Boot REST → Axum 完全変換パターン
///
/// Spring MVC アノテーション → Axum ルーター変換の参照実装。
/// この実装は cargo check / cargo test が通る完全な Axum アプリです。
///
/// 変換テーブル:
///   @RestController           → Router::new() を返す関数
///   @RequestMapping("/api")   → .nest("/api", routes())
///   @GetMapping("/{id}")      → .route("/{id}", get(handler))
///   @PostMapping              → .route("/", post(handler))
///   @PutMapping("/{id}")      → .route("/{id}", put(handler))
///   @DeleteMapping("/{id}")   → .route("/{id}", delete(handler))
///   @PathVariable Long id     → Path(id): Path<i64>
///   @RequestBody Article a    → Json(a): Json<Article>
///   @RequestParam String name → Query(q): Query<Params>
///   @RequestHeader String h   → headers: HeaderMap
///   ResponseEntity<T> (200)   → Json<T>
///   ResponseEntity<T> (201)   → (StatusCode::CREATED, Json<T>)
///   @Autowired Service svc    → State(svc): State<Arc<Service>>
use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::get,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

// ───────────────────────────────────────────────
// Java POJO / record → Rust struct
// ───────────────────────────────────────────────

/// Java:
///   public record Article(Long id, String title, String body) {}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Article {
    pub id: Option<i64>,
    pub title: String,
    pub body: String,
}

/// Java: @RequestParam String query (デフォルト値付き)
#[derive(Debug, Deserialize)]
pub struct ArticleQuery {
    #[serde(default)]
    pub query: String,
    #[serde(default = "default_page")]
    pub page: u32,
}

fn default_page() -> u32 {
    1
}

// ───────────────────────────────────────────────
// @Service / @Repository → plain struct
// ───────────────────────────────────────────────

/// Java:
///   @Service
///   public class ArticleService {
///     private final List<Article> store = new ArrayList<>();
///     ...
///   }
pub struct ArticleService {
    store: Mutex<Vec<Article>>,
    next_id: Mutex<i64>,
}

impl ArticleService {
    pub fn new() -> Self {
        Self {
            store: Mutex::new(Vec::new()),
            next_id: Mutex::new(1),
        }
    }

    /// Java: article.findById(id) → Optional<Article>
    pub fn find_by_id(&self, id: i64) -> Option<Article> {
        self.store
            .lock()
            .unwrap()
            .iter()
            .find(|a| a.id == Some(id))
            .cloned()
    }

    /// Java: repository.findAll() → List<Article>
    pub fn find_all(&self, query: &str) -> Vec<Article> {
        let store = self.store.lock().unwrap();
        if query.is_empty() {
            store.clone()
        } else {
            store
                .iter()
                .filter(|a| a.title.contains(query) || a.body.contains(query))
                .cloned()
                .collect()
        }
    }

    /// Java: repository.save(article) → Article
    pub fn save(&self, mut article: Article) -> Article {
        let mut id_guard = self.next_id.lock().unwrap();
        article.id = Some(*id_guard);
        *id_guard += 1;
        drop(id_guard);
        self.store.lock().unwrap().push(article.clone());
        article
    }

    /// Java: repository.save(article) (update) → Optional<Article>
    pub fn update(&self, id: i64, updated: Article) -> Option<Article> {
        let mut store = self.store.lock().unwrap();
        if let Some(a) = store.iter_mut().find(|a| a.id == Some(id)) {
            a.title = updated.title;
            a.body = updated.body;
            return Some(a.clone());
        }
        None
    }

    /// Java: repository.deleteById(id) → boolean
    pub fn delete(&self, id: i64) -> bool {
        let mut store = self.store.lock().unwrap();
        let before = store.len();
        store.retain(|a| a.id != Some(id));
        store.len() < before
    }
}

impl Default for ArticleService {
    fn default() -> Self {
        Self::new()
    }
}

// ───────────────────────────────────────────────
// @RestController → Axum ハンドラ関数群
// ───────────────────────────────────────────────

/// Java:
///   @GetMapping
///   public ResponseEntity<List<Article>> listArticles(
///     @RequestParam(defaultValue = "") String query) { ... }
pub async fn list_articles(
    State(svc): State<Arc<ArticleService>>,
    Query(params): Query<ArticleQuery>,
) -> Json<Vec<Article>> {
    Json(svc.find_all(&params.query))
}

/// Java:
///   @GetMapping("/{id}")
///   public ResponseEntity<Article> getArticle(@PathVariable Long id) { ... }
pub async fn get_article(
    State(svc): State<Arc<ArticleService>>,
    Path(id): Path<i64>,
) -> Result<Json<Article>, StatusCode> {
    svc.find_by_id(id).map(Json).ok_or(StatusCode::NOT_FOUND)
}

/// Java:
///   @PostMapping
///   public ResponseEntity<Article> createArticle(@RequestBody Article article) { ... }
pub async fn create_article(
    State(svc): State<Arc<ArticleService>>,
    Json(article): Json<Article>,
) -> (StatusCode, Json<Article>) {
    let created = svc.save(article);
    (StatusCode::CREATED, Json(created))
}

/// Java:
///   @PutMapping("/{id}")
///   public ResponseEntity<Article> updateArticle(
///     @PathVariable Long id, @RequestBody Article article) { ... }
pub async fn update_article(
    State(svc): State<Arc<ArticleService>>,
    Path(id): Path<i64>,
    Json(article): Json<Article>,
) -> Result<Json<Article>, StatusCode> {
    svc.update(id, article)
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

/// Java:
///   @DeleteMapping("/{id}")
///   public ResponseEntity<Void> deleteArticle(@PathVariable Long id) { ... }
pub async fn delete_article(
    State(svc): State<Arc<ArticleService>>,
    Path(id): Path<i64>,
) -> StatusCode {
    if svc.delete(id) {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}

// ───────────────────────────────────────────────
// @RestController + @RequestMapping → Router
// ───────────────────────────────────────────────

/// Java:
///   @RestController
///   @RequestMapping("/api/articles")
///   public class ArticleController { ... }
///
/// Rust: 全ルートをまとめた Router を返す関数
pub fn article_routes(svc: Arc<ArticleService>) -> Router {
    Router::new()
        .route("/", get(list_articles).post(create_article))
        .route(
            "/{id}",
            get(get_article).put(update_article).delete(delete_article),
        )
        .with_state(svc)
}

/// Java: @SpringBootApplication main() → tokio::main
pub fn build_app(svc: Arc<ArticleService>) -> Router {
    Router::new().nest("/api/articles", article_routes(svc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Method, Request},
    };
    use tower::ServiceExt; // for `.oneshot()`

    fn test_service() -> Arc<ArticleService> {
        Arc::new(ArticleService::new())
    }

    async fn body_to_string(body: axum::body::Body) -> String {
        use http_body_util::BodyExt;
        let bytes = body.collect().await.unwrap().to_bytes();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn test_list_articles_empty() {
        let svc = test_service();
        let app = build_app(svc);
        let req = Request::builder()
            .method(Method::GET)
            .uri("/api/articles")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_to_string(resp.into_body()).await;
        assert_eq!(body, "[]");
    }

    #[tokio::test]
    async fn test_create_and_get_article() {
        let svc = test_service();
        let app = build_app(Arc::clone(&svc));

        // POST /api/articles
        let article_json = r#"{"title":"Test","body":"Content"}"#;
        let req = Request::builder()
            .method(Method::POST)
            .uri("/api/articles")
            .header("content-type", "application/json")
            .body(Body::from(article_json))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = body_to_string(resp.into_body()).await;
        let created: Article = serde_json::from_str(&body).unwrap();
        assert_eq!(created.id, Some(1));
        assert_eq!(created.title, "Test");

        // GET /api/articles/1
        let req = Request::builder()
            .method(Method::GET)
            .uri("/api/articles/1")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_get_article_not_found() {
        let svc = test_service();
        let app = build_app(svc);
        let req = Request::builder()
            .method(Method::GET)
            .uri("/api/articles/999")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_update_article() {
        let svc = test_service();
        // Create first
        svc.save(Article {
            id: None,
            title: "Old".into(),
            body: "old".into(),
        });
        let app = build_app(Arc::clone(&svc));

        let req = Request::builder()
            .method(Method::PUT)
            .uri("/api/articles/1")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"title":"New","body":"new"}"#))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_to_string(resp.into_body()).await;
        let updated: Article = serde_json::from_str(&body).unwrap();
        assert_eq!(updated.title, "New");
    }

    #[tokio::test]
    async fn test_delete_article() {
        let svc = test_service();
        svc.save(Article {
            id: None,
            title: "To delete".into(),
            body: "".into(),
        });
        let app = build_app(Arc::clone(&svc));

        let req = Request::builder()
            .method(Method::DELETE)
            .uri("/api/articles/1")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_delete_article_not_found() {
        let svc = test_service();
        let app = build_app(svc);
        let req = Request::builder()
            .method(Method::DELETE)
            .uri("/api/articles/999")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_list_with_query() {
        let svc = test_service();
        svc.save(Article {
            id: None,
            title: "Rust lang".into(),
            body: "systems".into(),
        });
        svc.save(Article {
            id: None,
            title: "Python".into(),
            body: "scripting".into(),
        });
        let app = build_app(Arc::clone(&svc));

        let req = Request::builder()
            .method(Method::GET)
            .uri("/api/articles?query=Rust")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_to_string(resp.into_body()).await;
        let articles: Vec<Article> = serde_json::from_str(&body).unwrap();
        assert_eq!(articles.len(), 1);
        assert_eq!(articles[0].title, "Rust lang");
    }

    // ArticleService 単体テスト
    #[test]
    fn test_service_crud() {
        let svc = ArticleService::new();
        assert_eq!(svc.find_all("").len(), 0);

        let a = svc.save(Article {
            id: None,
            title: "T1".into(),
            body: "B1".into(),
        });
        assert_eq!(a.id, Some(1));

        let found = svc.find_by_id(1).unwrap();
        assert_eq!(found.title, "T1");

        svc.update(
            1,
            Article {
                id: None,
                title: "Updated".into(),
                body: "B2".into(),
            },
        );
        let updated = svc.find_by_id(1).unwrap();
        assert_eq!(updated.title, "Updated");

        assert!(svc.delete(1));
        assert!(svc.find_by_id(1).is_none());
    }
}
