/// Phase 2: Java CompletableFuture → tokio::spawn 変換パターン
///
/// 変換ルール:
///   CompletableFuture.supplyAsync(() -> x)       → tokio::spawn(async move { x })
///   future.thenApply(f)                          → .map() on async result
///   future.thenCompose(f)                        → async チェーン (await)
///   future.exceptionally(e -> fallback)          → match + Err arm
///   CompletableFuture.allOf(f1, f2)              → tokio::join!(f1, f2)
///   CompletableFuture.anyOf(f1, f2)              → tokio::select!
///   future.get() (blocking)                      → .await (非同期)
///   ExecutorService.submit(() -> task)           → tokio::spawn(async { task })
use anyhow::Result;
use tokio::sync::oneshot;

// ───────────────────────────────────────────────
// supplyAsync → tokio::spawn
// ───────────────────────────────────────────────

/// Java:
///   CompletableFuture<String> f = CompletableFuture.supplyAsync(() -> fetchData());
///   String result = f.get();
///
/// Rust: tokio::spawn でバックグラウンド実行
pub async fn supply_async<T, F>(task: F) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    // spawn_blocking は CPU バウンドタスクに適切
    tokio::task::spawn_blocking(task)
        .await
        .map_err(|e| anyhow::anyhow!("task panicked: {}", e))
}

// ───────────────────────────────────────────────
// thenApply → async map
// ───────────────────────────────────────────────

/// Java:
///   future.thenApply(data -> data.toUpperCase())
///
/// Rust: 直接 await して変換（future.thenApply は通常の関数合成）
pub async fn then_apply<T, R, F>(future: impl std::future::Future<Output = T>, f: F) -> R
where
    F: FnOnce(T) -> R,
{
    let value = future.await;
    f(value)
}

// ───────────────────────────────────────────────
// thenCompose → async chain (sequential await)
// ───────────────────────────────────────────────

/// Java:
///   future.thenCompose(data -> CompletableFuture.supplyAsync(() -> process(data)))
///
/// Rust: async ブロックで直接チェーン
pub async fn then_compose<T, R, Fut, F>(first: impl std::future::Future<Output = T>, f: F) -> R
where
    F: FnOnce(T) -> Fut,
    Fut: std::future::Future<Output = R>,
{
    let value = first.await;
    f(value).await
}

// ───────────────────────────────────────────────
// exceptionally → Result の Err arm
// ───────────────────────────────────────────────

/// Java:
///   future.exceptionally(ex -> "error: " + ex.getMessage())
///
/// Rust: Result::unwrap_or_else
pub async fn exceptionally<T, F>(
    future: impl std::future::Future<Output = Result<T>>,
    fallback: F,
) -> T
where
    F: FnOnce(anyhow::Error) -> T,
{
    match future.await {
        Ok(v) => v,
        Err(e) => fallback(e),
    }
}

// ───────────────────────────────────────────────
// CompletableFuture.allOf → tokio::try_join!
// ───────────────────────────────────────────────

/// Java:
///   CompletableFuture.allOf(f1, f2).thenRun(() -> ...)
///
/// Rust: tokio::join! で両方を並列実行
pub async fn all_of_two<A, B>(
    fa: impl std::future::Future<Output = A>,
    fb: impl std::future::Future<Output = B>,
) -> (A, B) {
    tokio::join!(fa, fb)
}

pub async fn all_of_three<A, B, C>(
    fa: impl std::future::Future<Output = A>,
    fb: impl std::future::Future<Output = B>,
    fc: impl std::future::Future<Output = C>,
) -> (A, B, C) {
    tokio::join!(fa, fb, fc)
}

// ───────────────────────────────────────────────
// timeout → tokio::time::timeout
// ───────────────────────────────────────────────

/// Java:
///   future.get(5, TimeUnit.SECONDS) → throws TimeoutException
///
/// Rust: tokio::time::timeout
pub async fn with_timeout<T>(
    duration: std::time::Duration,
    future: impl std::future::Future<Output = T>,
) -> Result<T> {
    tokio::time::timeout(duration, future)
        .await
        .map_err(|_| anyhow::anyhow!("operation timed out after {:?}", duration))
}

// ───────────────────────────────────────────────
// Channel-based notification (CountDownLatch 相当)
// ───────────────────────────────────────────────

/// Java: CountDownLatch(1) + await() / countDown()
/// Rust: oneshot channel
pub fn countdown_latch() -> (oneshot::Sender<()>, oneshot::Receiver<()>) {
    oneshot::channel()
}

// ───────────────────────────────────────────────
// 完全なパイプライン変換例
// ───────────────────────────────────────────────

/// Java の CompletableFuture パイプライン全体の変換例:
///
/// ```java
/// CompletableFuture<String> result = CompletableFuture
///     .supplyAsync(() -> fetchData())
///     .thenApply(data -> process(data))
///     .exceptionally(ex -> "error: " + ex.getMessage());
/// ```
///
/// Rust:
pub async fn full_pipeline_example() -> String {
    let result: Result<String> = async {
        // supplyAsync → spawn_blocking for CPU-bound work
        let data = tokio::task::spawn_blocking(fetch_data_sync)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        // thenApply → synchronous transform
        Ok(process_data(data))
    }
    .await;

    // exceptionally → unwrap_or_else
    result.unwrap_or_else(|e| format!("error: {}", e))
}

fn fetch_data_sync() -> String {
    "raw_data".to_string()
}

fn process_data(data: String) -> String {
    data.to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_supply_async_basic() {
        // Java: CompletableFuture.supplyAsync(() -> 42).get() == 42
        let result = supply_async(|| 42i32).await.unwrap();
        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn test_supply_async_string() {
        // Java: CompletableFuture.supplyAsync(() -> "hello").get()
        let result = supply_async(|| "hello".to_string()).await.unwrap();
        assert_eq!(result, "hello");
    }

    #[tokio::test]
    async fn test_then_apply() {
        // Java: CompletableFuture.supplyAsync(() -> 5).thenApply(x -> x * 2).get() == 10
        let base = async { 5i32 };
        let result = then_apply(base, |x| x * 2).await;
        assert_eq!(result, 10);
    }

    #[tokio::test]
    async fn test_then_apply_string() {
        // Java: future.thenApply(s -> s.toUpperCase())
        let base = async { "hello".to_string() };
        let result = then_apply(base, |s| s.to_uppercase()).await;
        assert_eq!(result, "HELLO");
    }

    #[tokio::test]
    async fn test_then_compose() {
        // Java: future.thenCompose(x -> CompletableFuture.supplyAsync(() -> x + 1))
        let first = async { 10i32 };
        let result = then_compose(first, |x| async move { x + 1 }).await;
        assert_eq!(result, 11);
    }

    #[tokio::test]
    async fn test_exceptionally_ok_path() {
        // Java: future.exceptionally(e -> -1) — 成功パスは値を返す
        let success = async { Ok::<i32, anyhow::Error>(42) };
        let result = exceptionally(success, |_| -1).await;
        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn test_exceptionally_error_path() {
        // Java: future.exceptionally(e -> "error: " + e.getMessage())
        let failure = async { Err::<String, _>(anyhow::anyhow!("network error")) };
        let result = exceptionally(failure, |e| format!("error: {}", e)).await;
        assert!(result.starts_with("error: network error"));
    }

    #[tokio::test]
    async fn test_all_of_two() {
        // Java: CompletableFuture.allOf(f1, f2)
        let f1 = async { 1i32 };
        let f2 = async { 2i32 };
        let (a, b) = all_of_two(f1, f2).await;
        assert_eq!(a, 1);
        assert_eq!(b, 2);
    }

    #[tokio::test]
    async fn test_all_of_three() {
        // Java: CompletableFuture.allOf(f1, f2, f3)
        let (a, b, c) = all_of_three(async { 1 }, async { 2 }, async { 3 }).await;
        assert_eq!((a, b, c), (1, 2, 3));
    }

    #[tokio::test]
    async fn test_with_timeout_ok() {
        // Java: future.get(1, TimeUnit.SECONDS) → 成功
        let fast = async { "done".to_string() };
        let result = with_timeout(std::time::Duration::from_secs(1), fast)
            .await
            .unwrap();
        assert_eq!(result, "done");
    }

    #[tokio::test]
    async fn test_full_pipeline() {
        // Java 全パイプライン変換例
        let result = full_pipeline_example().await;
        assert_eq!(result, "RAW_DATA");
    }

    #[tokio::test]
    async fn test_countdown_latch() {
        // Java: CountDownLatch の oneshot channel 相当
        let (tx, rx) = countdown_latch();
        let handle = tokio::spawn(async move {
            rx.await.unwrap();
            "signaled"
        });
        tx.send(()).unwrap();
        let result = handle.await.unwrap();
        assert_eq!(result, "signaled");
    }
}
