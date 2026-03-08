# QA #09 レビュー — java-to-rust feat/guava-async-proptest

- **レビュー日**: 2026-03-08
- **PR**: feat/guava-async-proptest → main
- **担当**: #07
- **レビュワー**: QA #09

---

## 判定: **CONDITIONAL APPROVAL（条件付き承認）**

**条件**:
1. `git rm -r --cached output/spring-axum/` で output/ 追跡解除

---

## チェックリスト

| 項目 | 結果 |
|------|------|
| CI: `RUSTFLAGS="-Dwarnings" cargo check --workspace` | ✅ PASS |
| CI: `cargo test --workspace` | ✅ PASS — 101テスト |
| CI: `cargo clippy --workspace -- -D warnings` | ✅ PASS — 警告なし |
| CI: `cargo fmt --all -- --check` | ✅ PASS |
| `output/` が `.gitignore` に追加されている | ✅ PASS |
| `output/` がgit追跡から除外されている | ❌ **BLOCKER** — output/spring-axum/ 追跡中 |
| `unsafe` の不必要な使用なし | ✅ PASS |

---

## output/ 追跡解除が必要なファイル

```
output/spring-axum/Cargo.toml
output/spring-axum/src/main.rs
```

```bash
git rm -r --cached output/
git commit -m "chore: remove tracked output/ files (gitignored)"
git push
```

---

## 良い点

- **ImmutableList → Vec**: `ImmutableList.of(1,2,3)` → `vec![1,2,3]` 正確に変換
- **CompletableFuture → tokio::join!**:
  - `allOf(f1,f2)` → `tokio::join!(fa, fb)` ✓ (`async_patterns.rs:101`)
  - `allOf(f1,f2,f3)` → `tokio::join!(fa, fb, fc)` ✓ (`async_patterns.rs:109`)
- **@Nested → mod {}**: JUnit 5 ネストクラス → Rustモジュール変換 ✓ (`proptest_patterns.rs:12`)
- **Spring MVC → Axum 0.8**:
  - `@PostMapping("/{id}")` → `.route("/{id}", post(handler))` ✓（`/{id}` 構文、`/:id` でない）
  - Axum 0.8 パスパラメータ構文を正しく使用
- **101テスト全通過**: Guava/async/proptest/Spring変換を網羅

---

## アクション

1. `git rm -r --cached output/spring-axum/` で追跡解除してコミット
2. `git push` して再レビュー依頼

修正確認後 APPROVED とします。

---
*QA #09 — 2026-03-08*
