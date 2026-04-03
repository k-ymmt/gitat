# TODO

## 未実装機能（MVPスコープ）

- [ ] 検索モード (`/`) — `Mode::Search` に入るがフィルタリングロジックが未実装
- [ ] コンフリクトエディタのインライン編集 (`e` キー) — `editing` フィールドは存在するがキーハンドラなし
- [ ] Diff のコンテキスト折りたたみ — 変更のない領域を折りたたんで表示

## コミット詳細画面の改善

- [ ] `FileChangeStatus` に `Copied` バリアントを追加 — `git diff-tree` の `C` ステータスが現在 `Modified` にフォールバックしている
- [ ] リネーム時の旧パス情報を保持 — `CommitFileEntry` に `old_path: Option<String>` を追加し「旧名 → 新名」表示を可能にする
- [ ] j/k ナビゲーション時の不要な diff リロードを回避 — 選択が変わらない場合はスキップする最適化

## リファクタリング

- [x] `is_staged` 判定ロジックの重複排除 — `StatusEntry::is_staged()` メソッドとして `gitat-core` に抽出済み
- [x] `event.rs` のモジュール分割 — `event/mod.rs`, `normal.rs`, `staging.rs`, `commit_detail.rs` に分割済み
- [x] `stage_or_unstage_hunk` の diff リロード失敗時にステータスメッセージでユーザーに通知

## テスト

- [ ] 統合テスト: ステージング後の diff 読み込み、タブ切り替え時の状態保持
- [ ] ビュー全体のスナップショットテスト（`views/status`, `views/log`, `views/branches` の複合レイアウト）
- [ ] コミット詳細画面のスナップショットテスト（メタデータ + ファイル一覧 + diff パネルのレイアウト）
- [ ] `assert_yaml_snapshot!` への移行検討（serde を dev-dep に追加してスナップショットの可読性向上）
- [ ] `cargo-insta` CLI 導入（`cargo insta review` による対話的スナップショット承認）

## Post-MVP

- [ ] Interactive rebase UI
- [ ] Stash 管理（タブはプレースホルダーのみ）
- [ ] Git submodule サポート
- [ ] 設定ファイル / カスタムキーバインド
- [ ] Diff のシンタックスハイライト
- [ ] 非同期 git コマンド実行（長時間操作をバックグラウンドスレッドに移動）
