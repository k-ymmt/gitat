# TODO

## バグ / 要修正

- [x] `push`/`pull` が `"origin", "HEAD"` を使用している — 現在のブランチ名を推論すべき
- [ ] ステータスメッセージが自動消去されない（スペックでは3秒で消去）
- [x] ブランチ解析で `last_commit` にコミットメッセージが入り、short hash がスキップされている
- [x] イベントハンドラで毎キー入力時に `mode.clone()` が発生 — 参照またはdiscriminantマッチに変更すべき
- [x] `centered_rect` が `views/commit.rs` と `main.rs` で重複 — 共通ユーティリティに抽出すべき

## 未実装機能（MVPスコープ）

- [ ] 検索モード (`/`) — `Mode::Search` に入るがフィルタリングロジックが未実装
- [ ] コンフリクトエディタのインライン編集 (`e` キー) — `editing` フィールドは存在するがキーハンドラなし
- [ ] Diff のコンテキスト折りたたみ — 変更のない領域を折りたたんで表示
- [ ] `stage_hunk` — スペックに記載があるが `stage_file`/`unstage_file` のみ実装
- [x] Resize イベントハンドリング — スペックでは `Event::Resize` を処理しているが `main.rs` は `Event::Key` のみ

## テスト

- [ ] `insta` スナップショットテストを追加（diff ウィジェットやビュー向け、dev-dep は追加済み）
- [ ] 統合テスト: ステージング後の diff 読み込み、タブ切り替え時の状態保持

## Post-MVP

- [ ] Interactive rebase UI
- [ ] Stash 管理（タブはプレースホルダーのみ）
- [ ] Git submodule サポート
- [ ] 設定ファイル / カスタムキーバインド
- [ ] Diff のシンタックスハイライト
- [ ] 非同期 git コマンド実行（長時間操作をバックグラウンドスレッドに移動）
