//! 各サブコマンド（`lint`/`fmt`/`analyze`/`list`/`describe`/`init`）の実行ロジック。
//!
//! 第13弾: `src/main.rs`のSRP分割で、CLI定義（`src/cli.rs`）・ファイル収集
//! ユーティリティ（`src/fs_walk.rs`）から独立させ、サブコマンド1つにつき
//! 1モジュールとして切り出した。`src/main.rs`本体は引数パース→ディスパッチのみの
//! 薄いエントリポイントになる。

pub mod analyze;
pub mod common;
pub mod describe;
pub mod fmt;
pub mod init;
pub mod lint;
pub mod list;
