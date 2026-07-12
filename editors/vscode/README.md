# Simutrans dat_linter (VSCode 拡張)

Simutrans アドオンの `.dat` を色分けして見やすくしたり、フォーマット、パラメーターの不備をチェックできる拡張です。

![syntax highlighting](./docs/img1.png)

# 機能

## シンタックスハイライト

`obj=building` などのキーを見やすく色分けします。

## スニペット

`buidling=res` などobj形式を入力するとそのあアドオンのテンプレートを生成できます。

## フォーマット ※

パラメーターの順序を整えたり大文字・小文字を整えます。

## パラメーターチェック（lint） ※

パラーメーターの不足や値のミスを指摘します。

![syntax highlighting](./docs/img4.png)

## 言語切り替え ※

英語・日本語表示の切り替えができます。

![syntax highlighting](./docs/img2.png)
![syntax highlighting](./docs/img3.png)

## 依存ツール

※の機能を使うには [`dat_linter`](https://github.com/128na/simutrans-dat-linter) の導入が必要です。

この拡張は `dat_linter` 本体を同梱しません。事前に別途インストールし、PATH に通しておく必要があります。

- 本体・インストール方法: https://github.com/128na/simutrans-dat-linter
  （リリースページから OS にあった実行ファイルをダウンロードしてください）
- `--format json` オプションは `dat_linter 0.1.2` 以降が対応しています。それより古いバージョンでは
  この拡張は動作しません（後述「既知の制限」参照）。

## 旧拡張(`128na/simutrans-vscode-extention`)からの移行

作者（128na）が以前公開していた別のVSCode拡張
[`128na/simutrans-vscode-extention`](https://github.com/128na/simutrans-vscode-extention)（CC0）にも、
`.dat` 向けのシンタックスハイライト・スニペットが含まれています。両拡張がともに `.dat` に対して
言語定義・グラマーを提供するため、**両方を同時にインストールしていると、どちらの言語ID/グラマーが
実際に使われるかVSCode側の解決順に依存し、ハイライト表示が不安定になることがあります**。

同一 publisher（128na）による後継として、この拡張の採用後は旧拡張のアンインストールを推奨します。
lint（Problems パネル表示）・Document Formatting の機能は言語ID に依存しない実装
（`{pattern: "**/*.dat"}` によるファイル名ベースのセレクタ）のため、旧拡張を入れたままにするか
どうかに関わらず、これらの機能自体は影響を受けません。
