# TODO

## 未実装機能（MVPスコープ）

- [ ] 検索モード (`/`) — `Mode::Search` に入るがフィルタリングロジックが未実装
- [ ] コンフリクトエディタのインライン編集 (`e` キー) — `editing` フィールドは存在するがキーハンドラなし
- [ ] Diff のコンテキスト折りたたみ — 変更のない領域を折りたたんで表示

## UncommittedDetail ビューの改善

- [ ] ファイルリストのセクションヘッダーとデータインデックスの乖離を修正 — "Staged"/"Modified"/"Untracked" ヘッダーがリストアイテムとして追加されており、カーソルインデックスと `app.status` のインデックスが一致しない（旧 Status ビューから引き継いだ問題）
- [ ] staged/unstaged カウント計算ロジックの重複を解消 — `render_log_list` と `render_uncommitted_detail` で同じフィルタリングが行われている。`App` にヘルパーメソッドを追加するか共通関数に抽出する
- [ ] Help モード（`?`）からの復帰先を修正 — UncommittedDetail から Help を開いて閉じると `Mode::Normal` に戻ってしまう。前モードを保存して正しく復帰させる
- [ ] `load_diff_for_selected` のデッドコードパスを整理 — `normal.rs` の Enter ハンドラで非 Log タブ時に呼ばれるが、関数先頭で `Mode::UncommittedDetail` チェックにより即 return する

## コミット詳細画面の改善

- [ ] マージコミットで第二親との差分表示を切り替え可能にする — 現在は常に第一親との差分のみ表示。GitHub のように比較対象の親を選択できると便利
- [ ] マージコミットであることの UI 表示 — コミット詳細画面のメタデータにマージコミットかどうか・親コミット情報を表示する
- [ ] `FileChangeStatus` に `Copied` バリアントを追加 — `git diff-tree` の `C` ステータスが現在 `Modified` にフォールバックしている
- [ ] リネーム時の旧パス情報を保持 — `CommitFileEntry` に `old_path: Option<String>` を追加し「旧名 → 新名」表示を可能にする
- [ ] j/k ナビゲーション時の不要な diff リロードを回避 — 選択が変わらない場合はスキップする最適化

## Log プレビューパネルの改善

- [ ] プレビューデータのキャッシュ — カーソル移動のたびに git コマンドを実行しており、大きなリポジトリで遅延の可能性がある。コミットハッシュベースのキャッシュを検討
- [ ] プレビューローダーのエラー時の状態クリア — `load_commit_preview` / `load_uncommitted_preview` で git コマンドが失敗した場合、古いプレビューデータが残り続ける。エラー時に `commit_detail_*` / `current_diff` をクリアすべき
- [ ] `load_commit_preview` と `enter_commit_detail` の重複ロジックの共通化 — コミット取得→ファイルリスト取得のパターンが `commit_detail.rs` 内で重複している。共通ヘルパーに抽出可能だがエラーハンドリングの差異（プレビューは静かにリターン、フルスクリーンは status_message を表示）に注意

## Unified Diff ウィジェットの改善

- [ ] `hunk_offsets` / `scroll_y` の u16 オーバーフロー対策 — 1カラム表示では DiffRow が最大2行に展開されるため、巨大な diff で u16::MAX (65535) を超える可能性がある
- [ ] `ScreenRow` 構造体を `render` メソッド外に抽出 — 現在ローカル構造体として定義されているが、モジュールレベルに移すことで展開ロジックの個別テストが可能になる
- [ ] `snapshot_render_narrow_terminal` テストの改善 — 1カラムレイアウトではコンテンツ幅が広くなり、現在のテストデータ (width=30) では切り詰めが発生しない。より長いコンテンツか狭い幅 (例: width=16) にする

## テスト

- [ ] 統合テスト: ステージング後の diff 読み込み、タブ切り替え時の状態保持
- [ ] ビュー全体のスナップショットテスト（`views/log` の UncommittedDetail、`views/branches` の複合レイアウト）
- [ ] コミット詳細画面のスナップショットテスト（メタデータ + ファイル一覧 + diff パネルのレイアウト）
- [ ] Log プレビューパネルのスナップショットテスト（commit preview / uncommitted preview の描画）
- [ ] コミット成功後に `Mode::UncommittedDetail` に戻ることの確認テスト
- [ ] `assert_yaml_snapshot!` への移行検討（serde を dev-dep に追加してスナップショットの可読性向上）
- [ ] `cargo-insta` CLI 導入（`cargo insta review` による対話的スナップショット承認）

## ファイルシステム監視の改善

- [ ] `.gitignore` ベースのパスフィルタリング — `notify` は `.gitignore` を認識しないため `node_modules/`, `target/` 等の変更でもイベントが発火する。`ignore` クレート等でフィルタリングを追加すれば不要な `git status` 実行を削減できる
- [ ] gitat 自身の操作後の冗長リフレッシュ抑制 — stage/commit 等の操作直後に `app.refresh()` と fs watcher からの `refresh_status_and_log()` が二重に走る。短いクールダウンフラグで抑制可能
- [ ] `spawn_blocking` の `JoinHandle` 保持 — 現在は戻り値を捨てている。将来的なグレースフルシャットダウン対応のため保持を検討
- [ ] ブランチ変更の自動監視 — 現在 `refresh_status_and_log()` は `branches` を更新しない。`.git/refs/heads/` や `.git/HEAD` の変更時にブランチも更新する拡張

## Post-MVP

- [ ] Interactive rebase UI
- [ ] Stash 管理（タブはプレースホルダーのみ）
- [ ] Git submodule サポート
- [ ] 設定ファイル / カスタムキーバインド
- [ ] Diff のシンタックスハイライト
- [ ] 非同期 git コマンド実行（長時間操作をバックグラウンドスレッドに移動）— メインイベントループは tokio 化済み。git コマンド自体のバックグラウンド実行が残
