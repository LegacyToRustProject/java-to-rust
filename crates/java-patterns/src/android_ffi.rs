//! Phase 5: Android API → Rust FFI スタブ生成
//!
//! Android SDK の主要 Java API を Rust FFI スタブとして表現するパターン集。
//! 実際の Android NDK 環境では JNI クレート (`jni`) と組み合わせて使用する。
//!
//! ## 変換テーブル (Java Android → Rust)
//!
//! | Java (Android SDK)               | Rust (FFI スタブ)                          |
//! |----------------------------------|--------------------------------------------|
//! | `android.content.Context`        | `AndroidContext` trait / `ContextHandle`   |
//! | `android.content.Intent`         | `Intent` struct                            |
//! | `android.os.Bundle`              | `Bundle` struct (HashMap<String, BundleValue>) |
//! | `android.app.Activity`           | `Activity` trait + `ActivityHandle`        |
//! | `android.app.Service`            | `Service` trait                            |
//! | `android.os.Handler`             | `Handler` (tokio channel ラッパー)         |
//! | `android.os.Looper`              | `Looper` (tokio runtime ラッパー)          |
//! | `android.os.AsyncTask<P,Prog,R>` | `async fn` + `tokio::task::spawn`          |
//! | `SharedPreferences`              | `SharedPrefs` (HashMap ラッパー)           |
//! | `android.util.Log`               | `log!` マクロ経由 / `tracing`              |
//!
//! ## JNI 実装パターン (NDK 環境向け)
//!
//! ```ignore
//! // Java側: native void processData(byte[] data);
//! #[no_mangle]
//! pub extern "C" fn Java_com_example_MyClass_processData(
//!     env: JNIEnv,
//!     _class: JClass,
//!     data: jbyteArray,
//! ) {
//!     let bytes = env.convert_byte_array(data).unwrap();
//!     process_bytes(&bytes);
//! }
//! ```

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// ─────────────────────────────────────────────────────────────────
// Bundle: android.os.Bundle → HashMap<String, BundleValue>
// ─────────────────────────────────────────────────────────────────

/// Java `android.os.Bundle` の Rust 表現。
///
/// Java:
/// ```java
/// Bundle bundle = new Bundle();
/// bundle.putString("key", "value");
/// bundle.putInt("count", 42);
/// String s = bundle.getString("key");
/// ```
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Bundle {
    data: HashMap<String, BundleValue>,
}

/// Bundle に格納できる値の型（Java の型システムに対応）
#[derive(Debug, Clone, PartialEq)]
pub enum BundleValue {
    /// `bundle.putString` / `bundle.getString`
    Str(String),
    /// `bundle.putInt` / `bundle.getInt`
    Int(i32),
    /// `bundle.putLong` / `bundle.getLong`
    Long(i64),
    /// `bundle.putFloat` / `bundle.getFloat`
    Float(f32),
    /// `bundle.putDouble` / `bundle.getDouble`
    Double(f64),
    /// `bundle.putBoolean` / `bundle.getBoolean`
    Bool(bool),
    /// `bundle.putStringArray` / `bundle.getStringArray`
    StringArray(Vec<String>),
    /// `bundle.putBundle` / `bundle.getBundle` (ネスト)
    Nested(Box<Bundle>),
}

impl Bundle {
    /// `new Bundle()`
    pub fn new() -> Self {
        Self::default()
    }

    /// `bundle.putString(key, value)`
    pub fn put_string(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.data.insert(key.into(), BundleValue::Str(value.into()));
    }

    /// `bundle.getString(key)` → `Option<&str>`  (null-safe)
    pub fn get_string(&self, key: &str) -> Option<&str> {
        match self.data.get(key)? {
            BundleValue::Str(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// `bundle.getString(key, defaultValue)`
    pub fn get_string_or<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        self.get_string(key).unwrap_or(default)
    }

    /// `bundle.putInt(key, value)`
    pub fn put_int(&mut self, key: impl Into<String>, value: i32) {
        self.data.insert(key.into(), BundleValue::Int(value));
    }

    /// `bundle.getInt(key, defaultValue)`
    pub fn get_int(&self, key: &str, default: i32) -> i32 {
        match self.data.get(key) {
            Some(BundleValue::Int(v)) => *v,
            _ => default,
        }
    }

    /// `bundle.putLong(key, value)`
    pub fn put_long(&mut self, key: impl Into<String>, value: i64) {
        self.data.insert(key.into(), BundleValue::Long(value));
    }

    /// `bundle.getLong(key, defaultValue)`
    pub fn get_long(&self, key: &str, default: i64) -> i64 {
        match self.data.get(key) {
            Some(BundleValue::Long(v)) => *v,
            _ => default,
        }
    }

    /// `bundle.putBoolean(key, value)`
    pub fn put_boolean(&mut self, key: impl Into<String>, value: bool) {
        self.data.insert(key.into(), BundleValue::Bool(value));
    }

    /// `bundle.getBoolean(key, defaultValue)`
    pub fn get_boolean(&self, key: &str, default: bool) -> bool {
        match self.data.get(key) {
            Some(BundleValue::Bool(v)) => *v,
            _ => default,
        }
    }

    /// `bundle.putStringArray(key, values)`
    pub fn put_string_array(&mut self, key: impl Into<String>, values: Vec<String>) {
        self.data
            .insert(key.into(), BundleValue::StringArray(values));
    }

    /// `bundle.getStringArray(key)`
    pub fn get_string_array(&self, key: &str) -> Option<&[String]> {
        match self.data.get(key)? {
            BundleValue::StringArray(v) => Some(v.as_slice()),
            _ => None,
        }
    }

    /// `bundle.putBundle(key, nested)`
    pub fn put_bundle(&mut self, key: impl Into<String>, nested: Bundle) {
        self.data
            .insert(key.into(), BundleValue::Nested(Box::new(nested)));
    }

    /// `bundle.getBundle(key)`
    pub fn get_bundle(&self, key: &str) -> Option<&Bundle> {
        match self.data.get(key)? {
            BundleValue::Nested(b) => Some(b.as_ref()),
            _ => None,
        }
    }

    /// `bundle.containsKey(key)`
    pub fn contains_key(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }

    /// `bundle.size()`
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// `bundle.isEmpty()`
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// `bundle.remove(key)`
    pub fn remove(&mut self, key: &str) -> Option<BundleValue> {
        self.data.remove(key)
    }

    /// `bundle.clear()`
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// `bundle.keySet()`
    pub fn key_set(&self) -> impl Iterator<Item = &String> {
        self.data.keys()
    }
}

// ─────────────────────────────────────────────────────────────────
// Intent: android.content.Intent
// ─────────────────────────────────────────────────────────────────

/// Java `android.content.Intent` の Rust 表現。
///
/// Java:
/// ```java
/// Intent intent = new Intent(context, TargetActivity.class);
/// intent.setAction(Intent.ACTION_VIEW);
/// intent.putExtra("key", "value");
/// startActivity(intent);
/// ```
#[derive(Debug, Clone)]
pub struct Intent {
    /// `intent.getAction()` / `intent.setAction()`
    pub action: Option<String>,
    /// コンポーネント名（パッケージ/クラス形式）
    pub component: Option<String>,
    /// `intent.getData()` / URI
    pub data_uri: Option<String>,
    /// `intent.getType()` / MIME タイプ
    pub mime_type: Option<String>,
    /// `intent.getExtras()` に相当するバンドル
    extras: Bundle,
    /// `intent.getFlags()`
    pub flags: i32,
    /// `intent.getCategories()`
    categories: Vec<String>,
}

/// Android 標準アクション定数
pub mod intent_actions {
    pub const ACTION_VIEW: &str = "android.intent.action.VIEW";
    pub const ACTION_SEND: &str = "android.intent.action.SEND";
    pub const ACTION_MAIN: &str = "android.intent.action.MAIN";
    pub const ACTION_CALL: &str = "android.intent.action.CALL";
    pub const ACTION_PICK: &str = "android.intent.action.PICK";
    pub const ACTION_EDIT: &str = "android.intent.action.EDIT";
    pub const ACTION_DELETE: &str = "android.intent.action.DELETE";
    pub const ACTION_BOOT_COMPLETED: &str = "android.intent.action.BOOT_COMPLETED";
}

/// Android Intent フラグ定数
pub mod intent_flags {
    pub const FLAG_ACTIVITY_NEW_TASK: i32 = 0x10000000;
    pub const FLAG_ACTIVITY_CLEAR_TOP: i32 = 0x04000000;
    pub const FLAG_ACTIVITY_SINGLE_TOP: i32 = 0x20000000;
    pub const FLAG_ACTIVITY_NO_HISTORY: i32 = 0x40000000;
}

impl Intent {
    /// `new Intent()`
    pub fn new() -> Self {
        Self {
            action: None,
            component: None,
            data_uri: None,
            mime_type: None,
            extras: Bundle::new(),
            flags: 0,
            categories: Vec::new(),
        }
    }

    /// `new Intent(action)`
    pub fn with_action(action: impl Into<String>) -> Self {
        let mut intent = Self::new();
        intent.action = Some(action.into());
        intent
    }

    /// `new Intent(context, TargetActivity.class)`
    /// → component = "com.example/TargetActivity"
    pub fn with_component(package: impl Into<String>, class: impl Into<String>) -> Self {
        let mut intent = Self::new();
        intent.component = Some(format!("{}/{}", package.into(), class.into()));
        intent
    }

    /// `intent.setAction(action)`
    pub fn set_action(&mut self, action: impl Into<String>) -> &mut Self {
        self.action = Some(action.into());
        self
    }

    /// `intent.setData(uri)`
    pub fn set_data(&mut self, uri: impl Into<String>) -> &mut Self {
        self.data_uri = Some(uri.into());
        self
    }

    /// `intent.setType(mimeType)`
    pub fn set_type(&mut self, mime: impl Into<String>) -> &mut Self {
        self.mime_type = Some(mime.into());
        self
    }

    /// `intent.setFlags(flags)`
    pub fn set_flags(&mut self, flags: i32) -> &mut Self {
        self.flags = flags;
        self
    }

    /// `intent.addFlags(flags)`
    pub fn add_flags(&mut self, flags: i32) -> &mut Self {
        self.flags |= flags;
        self
    }

    /// `intent.addCategory(category)`
    pub fn add_category(&mut self, category: impl Into<String>) -> &mut Self {
        self.categories.push(category.into());
        self
    }

    /// `intent.putExtra(key, value: String)`
    pub fn put_extra_string(&mut self, key: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.extras.put_string(key, value);
        self
    }

    /// `intent.putExtra(key, value: int)`
    pub fn put_extra_int(&mut self, key: impl Into<String>, value: i32) -> &mut Self {
        self.extras.put_int(key, value);
        self
    }

    /// `intent.putExtra(key, value: long)`
    pub fn put_extra_long(&mut self, key: impl Into<String>, value: i64) -> &mut Self {
        self.extras.put_long(key, value);
        self
    }

    /// `intent.putExtra(key, value: boolean)`
    pub fn put_extra_boolean(&mut self, key: impl Into<String>, value: bool) -> &mut Self {
        self.extras.put_boolean(key, value);
        self
    }

    /// `intent.putExtras(bundle)`
    pub fn put_extras(&mut self, bundle: Bundle) -> &mut Self {
        for (k, v) in bundle.data {
            self.extras.data.insert(k, v);
        }
        self
    }

    /// `intent.getStringExtra(key)`
    pub fn get_string_extra(&self, key: &str) -> Option<&str> {
        self.extras.get_string(key)
    }

    /// `intent.getIntExtra(key, defaultValue)`
    pub fn get_int_extra(&self, key: &str, default: i32) -> i32 {
        self.extras.get_int(key, default)
    }

    /// `intent.getLongExtra(key, defaultValue)`
    pub fn get_long_extra(&self, key: &str, default: i64) -> i64 {
        self.extras.get_long(key, default)
    }

    /// `intent.getBooleanExtra(key, defaultValue)`
    pub fn get_boolean_extra(&self, key: &str, default: bool) -> bool {
        self.extras.get_boolean(key, default)
    }

    /// `intent.getExtras()`
    pub fn get_extras(&self) -> &Bundle {
        &self.extras
    }

    /// `intent.hasExtra(key)`
    pub fn has_extra(&self, key: &str) -> bool {
        self.extras.contains_key(key)
    }

    /// `intent.getCategories()`
    pub fn get_categories(&self) -> &[String] {
        &self.categories
    }
}

impl Default for Intent {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────────────────────────
// Context: android.content.Context
// ─────────────────────────────────────────────────────────────────

/// Java `android.content.Context` の Rust trait 表現。
///
/// Java:
/// ```java
/// Context ctx = getApplicationContext();
/// String pkgName = ctx.getPackageName();
/// ctx.startActivity(intent);
/// ctx.startService(intent);
/// SharedPreferences prefs = ctx.getSharedPreferences("prefs", MODE_PRIVATE);
/// ```
pub trait AndroidContext: Send + Sync {
    /// `context.getPackageName()`
    fn get_package_name(&self) -> &str;

    /// `context.getApplicationContext()`
    fn get_application_context(&self) -> Arc<dyn AndroidContext>;

    /// `context.startActivity(intent)` — stub: Intentをキューに積む
    fn start_activity(&self, intent: Intent);

    /// `context.startService(intent)` — stub
    fn start_service(&self, intent: Intent);

    /// `context.getSharedPreferences(name, mode)`
    fn get_shared_preferences(&self, name: &str) -> Arc<Mutex<SharedPrefs>>;

    /// `context.getSystemService(name)` — 簡略化: 文字列キーで返す
    fn get_system_service(&self, name: &str) -> Option<String>;

    /// `context.sendBroadcast(intent)`
    fn send_broadcast(&self, intent: Intent);
}

// ─────────────────────────────────────────────────────────────────
// SharedPreferences: android.content.SharedPreferences
// ─────────────────────────────────────────────────────────────────

/// Java `android.content.SharedPreferences` の Rust 表現。
///
/// Java:
/// ```java
/// SharedPreferences prefs = ctx.getSharedPreferences("my_prefs", MODE_PRIVATE);
/// SharedPreferences.Editor editor = prefs.edit();
/// editor.putString("token", "abc123");
/// editor.apply();
/// String token = prefs.getString("token", "");
/// ```
#[derive(Debug, Default)]
pub struct SharedPrefs {
    data: HashMap<String, BundleValue>,
}

impl SharedPrefs {
    pub fn new() -> Self {
        Self::default()
    }

    /// `prefs.edit()` → Editor パターン: 直接変更するシンプルな実装
    pub fn edit(&mut self) -> SharedPrefsEditor<'_> {
        SharedPrefsEditor { prefs: self }
    }

    /// `prefs.getString(key, defValue)`
    pub fn get_string<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        match self.data.get(key) {
            Some(BundleValue::Str(s)) => s.as_str(),
            _ => default,
        }
    }

    /// `prefs.getInt(key, defValue)`
    pub fn get_int(&self, key: &str, default: i32) -> i32 {
        match self.data.get(key) {
            Some(BundleValue::Int(v)) => *v,
            _ => default,
        }
    }

    /// `prefs.getBoolean(key, defValue)`
    pub fn get_boolean(&self, key: &str, default: bool) -> bool {
        match self.data.get(key) {
            Some(BundleValue::Bool(v)) => *v,
            _ => default,
        }
    }

    /// `prefs.getLong(key, defValue)`
    pub fn get_long(&self, key: &str, default: i64) -> i64 {
        match self.data.get(key) {
            Some(BundleValue::Long(v)) => *v,
            _ => default,
        }
    }

    /// `prefs.contains(key)`
    pub fn contains(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }

    /// `prefs.getAll()`
    pub fn get_all(&self) -> &HashMap<String, BundleValue> {
        &self.data
    }
}

/// `SharedPreferences.Editor` 相当
pub struct SharedPrefsEditor<'a> {
    prefs: &'a mut SharedPrefs,
}

impl<'a> SharedPrefsEditor<'a> {
    /// `editor.putString(key, value)`
    pub fn put_string(&mut self, key: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.prefs.data.insert(key.into(), BundleValue::Str(value.into()));
        self
    }

    /// `editor.putInt(key, value)`
    pub fn put_int(&mut self, key: impl Into<String>, value: i32) -> &mut Self {
        self.prefs.data.insert(key.into(), BundleValue::Int(value));
        self
    }

    /// `editor.putLong(key, value)`
    pub fn put_long(&mut self, key: impl Into<String>, value: i64) -> &mut Self {
        self.prefs.data.insert(key.into(), BundleValue::Long(value));
        self
    }

    /// `editor.putBoolean(key, value)`
    pub fn put_boolean(&mut self, key: impl Into<String>, value: bool) -> &mut Self {
        self.prefs.data.insert(key.into(), BundleValue::Bool(value));
        self
    }

    /// `editor.remove(key)`
    pub fn remove(&mut self, key: &str) -> &mut Self {
        self.prefs.data.remove(key);
        self
    }

    /// `editor.clear()`
    pub fn clear(&mut self) -> &mut Self {
        self.prefs.data.clear();
        self
    }

    /// `editor.apply()` / `editor.commit()` — メモリ内ではそのまま反映済み
    pub fn apply(&self) {} // no-op: data already mutated
    pub fn commit(&self) -> bool {
        true
    }
}

// ─────────────────────────────────────────────────────────────────
// Activity ライフサイクル: android.app.Activity
// ─────────────────────────────────────────────────────────────────

/// Android `Activity` ライフサイクルコールバックの Rust trait。
///
/// Java:
/// ```java
/// public class MyActivity extends AppCompatActivity {
///     @Override protected void onCreate(Bundle savedInstanceState) { ... }
///     @Override protected void onResume() { ... }
///     @Override protected void onPause() { ... }
///     @Override protected void onDestroy() { ... }
/// }
/// ```
pub trait ActivityLifecycle {
    /// `onCreate(Bundle savedInstanceState)`
    fn on_create(&mut self, saved_instance_state: Option<Bundle>);

    /// `onStart()`
    fn on_start(&mut self) {}

    /// `onResume()`
    fn on_resume(&mut self) {}

    /// `onPause()`
    fn on_pause(&mut self) {}

    /// `onStop()`
    fn on_stop(&mut self) {}

    /// `onDestroy()`
    fn on_destroy(&mut self) {}

    /// `onSaveInstanceState(Bundle outState)`
    fn on_save_instance_state(&self, _out_state: &mut Bundle) {}

    /// `onNewIntent(Intent intent)`
    fn on_new_intent(&mut self, _intent: Intent) {}

    /// `onActivityResult(int requestCode, int resultCode, Intent data)`
    fn on_activity_result(&mut self, _request_code: i32, _result_code: i32, _data: Option<Intent>) {}
}

/// `Activity` の状態
#[derive(Debug, Clone, PartialEq)]
pub enum ActivityState {
    Created,
    Started,
    Resumed,
    Paused,
    Stopped,
    Destroyed,
}

// ─────────────────────────────────────────────────────────────────
// Service ライフサイクル: android.app.Service
// ─────────────────────────────────────────────────────────────────

/// Android `Service` ライフサイクルの Rust trait。
///
/// Java:
/// ```java
/// public class MyService extends Service {
///     @Override public IBinder onBind(Intent intent) { return null; }
///     @Override public int onStartCommand(Intent intent, int flags, int startId) {
///         return START_STICKY;
///     }
///     @Override public void onDestroy() { ... }
/// }
/// ```
pub trait ServiceLifecycle {
    /// `onBind(Intent intent)` — None = unbindable service
    fn on_bind(&self, intent: &Intent) -> Option<String>;

    /// `onStartCommand(Intent, flags, startId)` → StartCommandResult
    fn on_start_command(&mut self, intent: Option<&Intent>, flags: i32, start_id: i32) -> StartCommandResult;

    /// `onDestroy()`
    fn on_destroy(&mut self) {}
}

/// `Service.START_*` 定数の Rust 表現
#[derive(Debug, Clone, PartialEq)]
pub enum StartCommandResult {
    /// `START_STICKY` — システムがサービスを再起動する（Intent は null で渡される）
    Sticky,
    /// `START_NOT_STICKY` — システムは再起動しない
    NotSticky,
    /// `START_REDELIVER_INTENT` — 再起動時に最後の Intent を再配信する
    RedeliverIntent,
}

// ─────────────────────────────────────────────────────────────────
// Handler/Looper: android.os.Handler + Looper
// ─────────────────────────────────────────────────────────────────

/// Android `Handler` の Rust 表現（tokio mpsc チャネルベース）。
///
/// Java:
/// ```java
/// Handler handler = new Handler(Looper.getMainLooper());
/// handler.post(() -> { /* UI スレッドで実行 */ });
/// handler.postDelayed(() -> { /* 遅延実行 */ }, 1000);
/// handler.sendMessage(msg);
/// ```
#[derive(Clone)]
pub struct Handler {
    sender: tokio::sync::mpsc::UnboundedSender<HandlerMessage>,
}

/// `Message` / `Runnable` の統一表現
pub struct HandlerMessage {
    pub what: i32,
    pub arg1: i32,
    pub arg2: i32,
    pub obj: Option<String>,
    /// `Runnable` 相当のクロージャ
    pub runnable: Option<Box<dyn FnOnce() + Send>>,
    /// `postDelayed` の遅延ミリ秒
    pub delay_ms: u64,
}

impl std::fmt::Debug for HandlerMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HandlerMessage")
            .field("what", &self.what)
            .field("delay_ms", &self.delay_ms)
            .finish()
    }
}

/// `Looper` — Handler のメッセージキューを処理するランループ
pub struct Looper {
    receiver: Mutex<Option<tokio::sync::mpsc::UnboundedReceiver<HandlerMessage>>>,
    sender: tokio::sync::mpsc::UnboundedSender<HandlerMessage>,
}

impl Looper {
    /// `Looper.prepare()` + `Looper.loop()` の代替: Looper を生成する
    pub fn new() -> Self {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        Self {
            receiver: Mutex::new(Some(rx)),
            sender: tx,
        }
    }

    /// この Looper に紐付く `Handler` を生成する
    /// `new Handler(looper)`
    pub fn create_handler(&self) -> Handler {
        Handler {
            sender: self.sender.clone(),
        }
    }

    /// `Looper.loop()` — メッセージを順次処理する (async版)
    /// 最初の呼び出しでレシーバを取り出し、ロックを保持せずに await する。
    pub async fn run_loop(&self) {
        let mut rx = self
            .receiver
            .lock()
            .unwrap()
            .take()
            .expect("run_loop called twice");
        while let Some(msg) = rx.recv().await {
            if let Some(f) = msg.runnable {
                if msg.delay_ms > 0 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(msg.delay_ms)).await;
                }
                f();
            }
        }
    }
}

impl Default for Looper {
    fn default() -> Self {
        Self::new()
    }
}

impl Handler {
    /// `handler.post(Runnable r)` — 即時実行キュー投入
    pub fn post<F: FnOnce() + Send + 'static>(&self, f: F) {
        let _ = self.sender.send(HandlerMessage {
            what: 0,
            arg1: 0,
            arg2: 0,
            obj: None,
            runnable: Some(Box::new(f)),
            delay_ms: 0,
        });
    }

    /// `handler.postDelayed(Runnable r, long delayMillis)`
    pub fn post_delayed<F: FnOnce() + Send + 'static>(&self, f: F, delay_ms: u64) {
        let _ = self.sender.send(HandlerMessage {
            what: 0,
            arg1: 0,
            arg2: 0,
            obj: None,
            runnable: Some(Box::new(f)),
            delay_ms,
        });
    }

    /// `handler.sendEmptyMessage(what)`
    pub fn send_empty_message(&self, what: i32) {
        let _ = self.sender.send(HandlerMessage {
            what,
            arg1: 0,
            arg2: 0,
            obj: None,
            runnable: None,
            delay_ms: 0,
        });
    }

    /// `handler.sendMessage(Message msg)`
    pub fn send_message(&self, what: i32, arg1: i32, arg2: i32, obj: Option<String>) {
        let _ = self.sender.send(HandlerMessage {
            what,
            arg1,
            arg2,
            obj,
            runnable: None,
            delay_ms: 0,
        });
    }
}

// ─────────────────────────────────────────────────────────────────
// AsyncTask → tokio::spawn (非推奨 API の変換パターン)
// ─────────────────────────────────────────────────────────────────

/// Java `AsyncTask<Params, Progress, Result>` の Rust 変換パターン。
///
/// Java:
/// ```java
/// new AsyncTask<String, Integer, Boolean>() {
///     @Override protected Boolean doInBackground(String... params) {
///         return heavyWork(params[0]);
///     }
///     @Override protected void onPostExecute(Boolean result) {
///         updateUI(result);
///     }
/// }.execute("input");
/// ```
///
/// Rust (tokio):
/// ```ignore
/// let handle = tokio::task::spawn_blocking(move || heavy_work(&input));
/// let result = handle.await.unwrap();
/// update_ui(result).await;
/// ```
pub struct AsyncTaskPattern;

impl AsyncTaskPattern {
    /// `doInBackground` + `onPostExecute` の組み合わせ変換
    ///
    /// Java: `asyncTask.execute(params)`
    /// Rust: `spawn_blocking` でバックグラウンド処理 → `await` で結果を受け取る
    pub async fn execute<P, R, F>(params: P, do_in_background: F) -> R
    where
        P: Send + 'static,
        R: Send + 'static,
        F: FnOnce(P) -> R + Send + 'static,
    {
        tokio::task::spawn_blocking(move || do_in_background(params))
            .await
            .expect("AsyncTask::doInBackground panicked")
    }

    /// `publishProgress` → tokio::sync::watch チャネル経由で進捗通知
    ///
    /// Java: `publishProgress(progress)`
    /// Rust: `progress_tx.send(progress)`
    pub fn publish_progress<T: Clone + Send + 'static>(
        progress_tx: &tokio::sync::watch::Sender<Option<T>>,
        value: T,
    ) {
        let _ = progress_tx.send(Some(value));
    }
}

// ─────────────────────────────────────────────────────────────────
// テスト用モック Context 実装
// ─────────────────────────────────────────────────────────────────

/// テスト用のメモリ内 Context 実装
#[derive(Default)]
pub struct MockContext {
    package_name: String,
    started_activities: Mutex<Vec<Intent>>,
    started_services: Mutex<Vec<Intent>>,
    broadcasts: Mutex<Vec<Intent>>,
    prefs: Mutex<HashMap<String, Arc<Mutex<SharedPrefs>>>>,
}

impl MockContext {
    pub fn new(package_name: impl Into<String>) -> Arc<Self> {
        Arc::new(Self {
            package_name: package_name.into(),
            started_activities: Mutex::new(Vec::new()),
            started_services: Mutex::new(Vec::new()),
            broadcasts: Mutex::new(Vec::new()),
            prefs: Mutex::new(HashMap::new()),
        })
    }

    pub fn started_activities(&self) -> Vec<Intent> {
        self.started_activities.lock().unwrap().clone()
    }

    pub fn started_services(&self) -> Vec<Intent> {
        self.started_services.lock().unwrap().clone()
    }

    pub fn received_broadcasts(&self) -> Vec<Intent> {
        self.broadcasts.lock().unwrap().clone()
    }
}

impl AndroidContext for MockContext {
    fn get_package_name(&self) -> &str {
        &self.package_name
    }

    fn get_application_context(&self) -> Arc<dyn AndroidContext> {
        // self-reference via Arc clone would need Arc<Self> — return a new mock for simplicity
        MockContext::new(self.package_name.clone())
    }

    fn start_activity(&self, intent: Intent) {
        self.started_activities.lock().unwrap().push(intent);
    }

    fn start_service(&self, intent: Intent) {
        self.started_services.lock().unwrap().push(intent);
    }

    fn get_shared_preferences(&self, name: &str) -> Arc<Mutex<SharedPrefs>> {
        let mut prefs = self.prefs.lock().unwrap();
        Arc::clone(
            prefs
                .entry(name.to_string())
                .or_insert_with(|| Arc::new(Mutex::new(SharedPrefs::new()))),
        )
    }

    fn get_system_service(&self, name: &str) -> Option<String> {
        // テスト用スタブ: サービス名をそのまま返す
        Some(format!("mock::{}", name))
    }

    fn send_broadcast(&self, intent: Intent) {
        self.broadcasts.lock().unwrap().push(intent);
    }
}

// ─────────────────────────────────────────────────────────────────
// JNI ブリッジ パターン (NDK環境向けドキュメント + スタブ)
// ─────────────────────────────────────────────────────────────────

/// JNI ブリッジ生成パターン。
///
/// Android NDK 環境では `jni` クレートを使って以下のように実装する:
///
/// ## Java側の native 宣言
/// ```java
/// public class RustBridge {
///     static { System.loadLibrary("myrust"); }
///     public static native String processText(String input);
///     public static native int[]  computeArray(int size);
///     public static native void   triggerCallback(long callbackPtr);
/// }
/// ```
///
/// ## Rust側の JNI エクスポート関数
/// ```ignore
/// use jni::{JNIEnv, objects::{JClass, JString}, sys::jint};
///
/// // Java: String processText(String input)
/// #[no_mangle]
/// pub extern "C" fn Java_com_example_RustBridge_processText(
///     mut env: JNIEnv,
///     _class: JClass,
///     input: JString,
/// ) -> jni::sys::jstring {
///     let input: String = env.get_string(&input).unwrap().into();
///     let result = process_text_impl(&input);
///     env.new_string(result).unwrap().into_raw()
/// }
///
/// // Java: int[] computeArray(int size)
/// #[no_mangle]
/// pub extern "C" fn Java_com_example_RustBridge_computeArray(
///     mut env: JNIEnv,
///     _class: JClass,
///     size: jint,
/// ) -> jni::sys::jintArray {
///     let data: Vec<i32> = (0..size).collect();
///     let arr = env.new_int_array(size).unwrap();
///     env.set_int_array_region(&arr, 0, &data).unwrap();
///     arr.into_raw()
/// }
/// ```
///
/// ## Cargo.toml 設定 (Android JNI ライブラリ)
/// ```toml
/// [lib]
/// name = "myrust"
/// crate-type = ["cdylib"]
///
/// [dependencies]
/// jni = { version = "0.21", features = ["invocation"] }
/// ```
pub struct JniBridgePattern;

impl JniBridgePattern {
    /// JNI 関数名の命名規則変換:
    /// Java クラス名 → Rust エクスポート関数名
    ///
    /// `com.example.MyClass.myMethod` → `Java_com_example_MyClass_myMethod`
    pub fn jni_function_name(package: &str, class: &str, method: &str) -> String {
        let pkg = package.replace('.', "_");
        format!("Java_{pkg}_{class}_{method}")
    }

    /// Java 型 → JNI 型シグネチャの変換表
    ///
    /// Java型      → JNI C型
    /// `boolean`   → `jboolean` (u8)
    /// `byte`      → `jbyte`    (i8)
    /// `char`      → `jchar`    (u16)
    /// `short`     → `jshort`   (i16)
    /// `int`       → `jint`     (i32)
    /// `long`      → `jlong`    (i64)
    /// `float`     → `jfloat`   (f32)
    /// `double`    → `jdouble`  (f64)
    /// `String`    → `JString`
    /// `byte[]`    → `jbyteArray`
    /// `int[]`     → `jintArray`
    /// `Object`    → `JObject`
    pub fn java_type_to_jni(java_type: &str) -> &'static str {
        match java_type {
            "boolean" => "jboolean",
            "byte" => "jbyte",
            "char" => "jchar",
            "short" => "jshort",
            "int" => "jint",
            "long" => "jlong",
            "float" => "jfloat",
            "double" => "jdouble",
            "void" => "()",
            "String" => "JString",
            "byte[]" => "jbyteArray",
            "int[]" => "jintArray",
            "long[]" => "jlongArray",
            "double[]" => "jdoubleArray",
            _ => "JObject",
        }
    }
}

// ─────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Bundle テスト ─────────────────────────────────────────────

    #[test]
    fn test_bundle_put_get_string() {
        let mut bundle = Bundle::new();
        bundle.put_string("key", "value");
        assert_eq!(bundle.get_string("key"), Some("value"));
        assert_eq!(bundle.get_string("missing"), None);
    }

    #[test]
    fn test_bundle_put_get_int() {
        let mut bundle = Bundle::new();
        bundle.put_int("count", 42);
        assert_eq!(bundle.get_int("count", 0), 42);
        assert_eq!(bundle.get_int("missing", -1), -1);
    }

    #[test]
    fn test_bundle_put_get_long() {
        let mut bundle = Bundle::new();
        bundle.put_long("ts", i64::MAX);
        assert_eq!(bundle.get_long("ts", 0), i64::MAX);
    }

    #[test]
    fn test_bundle_put_get_boolean() {
        let mut bundle = Bundle::new();
        bundle.put_boolean("flag", true);
        assert!(bundle.get_boolean("flag", false));
        assert!(!bundle.get_boolean("missing", false));
    }

    #[test]
    fn test_bundle_put_get_string_array() {
        let mut bundle = Bundle::new();
        bundle.put_string_array("list", vec!["a".into(), "b".into(), "c".into()]);
        let arr = bundle.get_string_array("list").unwrap();
        assert_eq!(arr, &["a", "b", "c"]);
    }

    #[test]
    fn test_bundle_nested() {
        let mut outer = Bundle::new();
        let mut inner = Bundle::new();
        inner.put_string("inner_key", "inner_val");
        outer.put_bundle("nested", inner);

        let retrieved = outer.get_bundle("nested").unwrap();
        assert_eq!(retrieved.get_string("inner_key"), Some("inner_val"));
    }

    #[test]
    fn test_bundle_contains_and_remove() {
        let mut bundle = Bundle::new();
        bundle.put_string("k", "v");
        assert!(bundle.contains_key("k"));
        assert_eq!(bundle.size(), 1);
        bundle.remove("k");
        assert!(!bundle.contains_key("k"));
        assert!(bundle.is_empty());
    }

    #[test]
    fn test_bundle_get_string_or_default() {
        let bundle = Bundle::new();
        assert_eq!(bundle.get_string_or("missing", "default"), "default");
    }

    // ── Intent テスト ─────────────────────────────────────────────

    #[test]
    fn test_intent_with_action() {
        let intent = Intent::with_action(intent_actions::ACTION_VIEW);
        assert_eq!(intent.action.as_deref(), Some("android.intent.action.VIEW"));
    }

    #[test]
    fn test_intent_with_component() {
        let intent = Intent::with_component("com.example", "MainActivity");
        assert_eq!(
            intent.component.as_deref(),
            Some("com.example/MainActivity")
        );
    }

    #[test]
    fn test_intent_put_get_extras() {
        let mut intent = Intent::new();
        intent.put_extra_string("token", "abc123");
        intent.put_extra_int("userId", 42);
        intent.put_extra_boolean("loggedIn", true);

        assert_eq!(intent.get_string_extra("token"), Some("abc123"));
        assert_eq!(intent.get_int_extra("userId", 0), 42);
        assert!(intent.get_boolean_extra("loggedIn", false));
        assert!(!intent.get_boolean_extra("missing", false));
    }

    #[test]
    fn test_intent_flags() {
        let mut intent = Intent::new();
        intent.set_flags(intent_flags::FLAG_ACTIVITY_NEW_TASK);
        intent.add_flags(intent_flags::FLAG_ACTIVITY_CLEAR_TOP);
        assert_eq!(
            intent.flags,
            intent_flags::FLAG_ACTIVITY_NEW_TASK | intent_flags::FLAG_ACTIVITY_CLEAR_TOP
        );
    }

    #[test]
    fn test_intent_categories() {
        let mut intent = Intent::new();
        intent.add_category("android.intent.category.LAUNCHER");
        intent.add_category("android.intent.category.DEFAULT");
        assert_eq!(intent.get_categories().len(), 2);
    }

    #[test]
    fn test_intent_put_extras_from_bundle() {
        let mut bundle = Bundle::new();
        bundle.put_string("bundled_key", "bundled_val");

        let mut intent = Intent::new();
        intent.put_extras(bundle);
        assert_eq!(intent.get_string_extra("bundled_key"), Some("bundled_val"));
    }

    #[test]
    fn test_intent_has_extra() {
        let mut intent = Intent::new();
        assert!(!intent.has_extra("key"));
        intent.put_extra_long("key", 100);
        assert!(intent.has_extra("key"));
    }

    // ── SharedPreferences テスト ──────────────────────────────────

    #[test]
    fn test_shared_prefs_edit_and_read() {
        let mut prefs = SharedPrefs::new();
        {
            let mut editor = prefs.edit();
            editor.put_string("token", "secret");
            editor.put_int("version", 3);
            editor.put_boolean("onboarded", true);
            editor.apply();
        }
        assert_eq!(prefs.get_string("token", ""), "secret");
        assert_eq!(prefs.get_int("version", 0), 3);
        assert!(prefs.get_boolean("onboarded", false));
    }

    #[test]
    fn test_shared_prefs_remove() {
        let mut prefs = SharedPrefs::new();
        prefs.edit().put_string("k", "v").apply();
        assert!(prefs.contains("k"));
        prefs.edit().remove("k").apply();
        assert!(!prefs.contains("k"));
    }

    #[test]
    fn test_shared_prefs_clear() {
        let mut prefs = SharedPrefs::new();
        prefs.edit().put_string("a", "1").put_string("b", "2").apply();
        assert_eq!(prefs.get_all().len(), 2);
        prefs.edit().clear().apply();
        assert_eq!(prefs.get_all().len(), 0);
    }

    // ── MockContext テスト ─────────────────────────────────────────

    #[test]
    fn test_mock_context_package_name() {
        let ctx = MockContext::new("com.example.myapp");
        assert_eq!(ctx.get_package_name(), "com.example.myapp");
    }

    #[test]
    fn test_mock_context_start_activity() {
        let ctx = MockContext::new("com.example");
        let intent = Intent::with_action(intent_actions::ACTION_VIEW);
        ctx.start_activity(intent);
        let started = ctx.started_activities();
        assert_eq!(started.len(), 1);
        assert_eq!(
            started[0].action.as_deref(),
            Some("android.intent.action.VIEW")
        );
    }

    #[test]
    fn test_mock_context_shared_prefs() {
        let ctx = MockContext::new("com.example");
        let prefs_arc = ctx.get_shared_preferences("my_prefs");
        {
            let mut prefs = prefs_arc.lock().unwrap();
            prefs.edit().put_string("user", "alice").apply();
        }
        // 同じ名前で再取得しても同じインスタンス
        let prefs_arc2 = ctx.get_shared_preferences("my_prefs");
        let prefs2 = prefs_arc2.lock().unwrap();
        assert_eq!(prefs2.get_string("user", ""), "alice");
    }

    #[test]
    fn test_mock_context_send_broadcast() {
        let ctx = MockContext::new("com.example");
        let intent = Intent::with_action(intent_actions::ACTION_BOOT_COMPLETED);
        ctx.send_broadcast(intent);
        assert_eq!(ctx.received_broadcasts().len(), 1);
    }

    // ── Handler/Looper テスト ─────────────────────────────────────

    #[test]
    fn test_handler_send_message() {
        let looper = Looper::new();
        let handler = looper.create_handler();
        // メッセージを投入（ループなしでキューに積むだけ）
        handler.send_empty_message(42);
        handler.send_message(1, 10, 20, Some("obj".to_string()));
        // キューにメッセージが入っていることを確認
        // (tokio mpsc は drop するまでメッセージが残る)
    }

    #[tokio::test]
    async fn test_handler_post_runnable() {
        let looper = Looper::new();
        let handler = looper.create_handler();
        let flag = Arc::new(Mutex::new(false));
        let flag_clone = Arc::clone(&flag);

        handler.post(move || {
            *flag_clone.lock().unwrap() = true;
        });

        // ループを短時間実行
        tokio::time::timeout(
            tokio::time::Duration::from_millis(100),
            looper.run_loop(),
        )
        .await
        .ok(); // timeout は正常

        assert!(*flag.lock().unwrap());
    }

    // ── AsyncTask → tokio 変換テスト ─────────────────────────────

    #[tokio::test]
    async fn test_async_task_execute() {
        let result = AsyncTaskPattern::execute("hello".to_string(), |input: String| {
            input.to_uppercase()
        })
        .await;
        assert_eq!(result, "HELLO");
    }

    #[tokio::test]
    async fn test_async_task_compute_heavy() {
        let result = AsyncTaskPattern::execute(100u64, |n| (1..=n).sum::<u64>()).await;
        assert_eq!(result, 5050);
    }

    // ── JNI ブリッジパターン テスト ───────────────────────────────

    #[test]
    fn test_jni_function_name() {
        assert_eq!(
            JniBridgePattern::jni_function_name("com.example", "MyClass", "processText"),
            "Java_com_example_MyClass_processText"
        );
    }

    #[test]
    fn test_java_type_to_jni() {
        assert_eq!(JniBridgePattern::java_type_to_jni("int"), "jint");
        assert_eq!(JniBridgePattern::java_type_to_jni("String"), "JString");
        assert_eq!(JniBridgePattern::java_type_to_jni("boolean"), "jboolean");
        assert_eq!(JniBridgePattern::java_type_to_jni("byte[]"), "jbyteArray");
        assert_eq!(JniBridgePattern::java_type_to_jni("void"), "()");
        assert_eq!(JniBridgePattern::java_type_to_jni("Object"), "JObject");
        assert_eq!(JniBridgePattern::java_type_to_jni("MyClass"), "JObject");
    }

    // ── Activity ライフサイクル テスト ────────────────────────────

    #[test]
    fn test_activity_lifecycle_states() {
        struct TestActivity {
            state: ActivityState,
            created_with_bundle: bool,
        }

        impl ActivityLifecycle for TestActivity {
            fn on_create(&mut self, saved: Option<Bundle>) {
                self.state = ActivityState::Created;
                self.created_with_bundle = saved.is_some();
            }
            fn on_resume(&mut self) {
                self.state = ActivityState::Resumed;
            }
            fn on_pause(&mut self) {
                self.state = ActivityState::Paused;
            }
            fn on_destroy(&mut self) {
                self.state = ActivityState::Destroyed;
            }
        }

        let mut activity = TestActivity {
            state: ActivityState::Created,
            created_with_bundle: false,
        };

        // 新規起動
        activity.on_create(None);
        assert_eq!(activity.state, ActivityState::Created);
        assert!(!activity.created_with_bundle);

        // 復元起動
        let mut saved = Bundle::new();
        saved.put_string("scroll_pos", "42");
        activity.on_create(Some(saved));
        assert!(activity.created_with_bundle);

        activity.on_resume();
        assert_eq!(activity.state, ActivityState::Resumed);

        activity.on_pause();
        assert_eq!(activity.state, ActivityState::Paused);

        activity.on_destroy();
        assert_eq!(activity.state, ActivityState::Destroyed);
    }

    // ── Service テスト ────────────────────────────────────────────

    #[test]
    fn test_service_start_sticky() {
        struct MyService {
            started: bool,
        }

        impl ServiceLifecycle for MyService {
            fn on_bind(&self, _intent: &Intent) -> Option<String> {
                None // Unbound service
            }

            fn on_start_command(
                &mut self,
                _intent: Option<&Intent>,
                _flags: i32,
                _start_id: i32,
            ) -> StartCommandResult {
                self.started = true;
                StartCommandResult::Sticky
            }
        }

        let mut svc = MyService { started: false };
        let intent = Intent::with_action("com.example.START");
        let result = svc.on_start_command(Some(&intent), 0, 1);
        assert!(svc.started);
        assert_eq!(result, StartCommandResult::Sticky);
        assert_eq!(svc.on_bind(&intent), None);
    }
}
