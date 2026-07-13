---
name: setup-dev-env
description: Windows環境でSimutransアドオン開発（dat_linter + VSCode拡張「Simutrans dat_linter」+ makeobj）に必要なツールを導入・設定するスキル。dat_linter本体の最新版ダウンロード・PATH追加、makeobj最新版のダウンロード・PATH追加（ビルド不要、公式GitHub Releaseのビルド済みexe/zipを使用）、旧VSCode拡張（128na.simutrans-dat-vscode-extention）のアンインストール、新VSCode拡張（128na.simutrans-dat-linter）のインストール・executablePath設定、の4項目について現在の導入状況を確認し、ユーザーに必要なものを選んでもらった上で導入する。「開発環境をセットアップして」「dat_linter入れて」「makeobj導入して」「VSCode拡張を整えて」「Simutransアドオン開発環境を作って」等と言われたら使う。Windows専用（macOS/Linuxでの利用は想定していない）。
---

# setup-dev-env — Simutransアドオン開発環境セットアップ（Windows）

Simutransアドオン開発（`.dat`作成・pak化）に必要な以下4項目を、状況確認 → ユーザーの選択 →
実行、の順で導入するスキル。**いきなり全部インストールしない**。必ず現状を確認し、
何を実行するかユーザーに選んでもらってから着手すること。

## 前提

- Windows専用。PowerShellでのコマンド実行を前提とする。
- ダウンロード・PATH変更・VSCode拡張の増減は、ユーザーへの明示的な確認を経てから
  実行すること（ダウンロードするファイル名・取得元URL・サイズを明示した上で確認を取る。
  グローバル運用規約の「Explicit permission required」に該当する操作）。
- リリース情報の取得は`Invoke-RestMethod`（PowerShell標準、追加インストール不要）を使う。
  `gh`（GitHub CLI）は非エンジニアのアドオン作者が入れている可能性が低いため使わない。

## Step 1: 現状確認

以下4項目それぞれについて、現在の導入状況を確認する。

### 1-1. dat_linter
```powershell
Get-Command dat_linter -ErrorAction SilentlyContinue
dat_linter -v   # 導入済みなら "dat_linter x.y.z" が出る（0.3.1以降のバージョンのみ対応。
                 # それより古い場合は -V か --version を使う）
```
最新版と比較する場合:
```powershell
(Invoke-RestMethod -Uri "https://api.github.com/repos/128na/simutrans-dat-linter/releases/latest").tag_name
```

### 1-2. makeobj
```powershell
Get-Command makeobj -ErrorAction SilentlyContinue
makeobj   # 引数無しで実行すると "Makeobj version 60.11 for Simutrans 124.5 and higher" のような
          # バージョン行が出力される（エラー終了するが、バージョン確認には十分）
```
最新版と比較する場合は下記「1-3の罠」を参照（`/releases/latest` はそのまま使えない）。

### 1-3. 旧VSCode拡張（アンインストール対象）
```powershell
code --list-extensions | Select-String -Pattern "128na.simutrans-dat-vscode-extention"
```

### 1-4. 新VSCode拡張
```powershell
code --list-extensions | Select-String -Pattern "128na.simutrans-dat-linter"
```
導入済みの場合、`executablePath`設定（未設定ならデフォルトの`"dat_linter"`＝PATH依存）も
確認するとよい:
```powershell
code --list-extensions --show-versions | Select-String -Pattern "128na.simutrans-dat-linter"
```

`code`コマンド自体が見つからない場合、VSCodeのコマンドパレットで
「Shell Command: Install 'code' command in PATH」を実行してもらう必要がある
（このスキルからは自動化できないので、ユーザーに案内すること）。

## Step 2: ユーザーに選択してもらう

Step 1の結果を要約して提示し、`AskUserQuestion`（`multiSelect: true`）で
「どれを実行するか」を選んでもらう。既に導入済み・最新の項目は選択肢の説明に
その旨を明記し、デフォルトでは選ばれない（＝再導入しない）方向で選択肢を組み立てる。

## Step 3: 実行

選ばれた項目だけを、以下の手順で実行する。**各ダウンロードの直前に、
ファイル名・取得元URL・保存先を明示してユーザーに確認を取ること。**

### 3-1. dat_linter導入
```powershell
$tag = (Invoke-RestMethod -Uri "https://api.github.com/repos/128na/simutrans-dat-linter/releases/latest").tag_name
$url = "https://github.com/128na/simutrans-dat-linter/releases/download/$tag/dat_linter-x86_64-pc-windows-msvc.exe"
$dest = "$env:LOCALAPPDATA\Programs\dat_linter"
New-Item -ItemType Directory -Force -Path $dest | Out-Null
Invoke-WebRequest -Uri $url -OutFile "$dest\dat_linter.exe"
```
PATH追加（ユーザースコープ、管理者権限不要）:
```powershell
$path = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($path -notlike "*$dest*") {
    [Environment]::SetEnvironmentVariable("PATH", "$path;$dest", "User")
}
```
PATH変更は**新しく開いたターミナル/VSCodeウィンドウから反映される**（既存セッションには
反映されない）ことをユーザーに案内すること。

### 3-2. makeobj導入

**既知の罠**: `simutrans/simutrans`リポジトリの`/releases/latest`は、常時更新される
"Nightly"タグ（テスト用・不安定）を返してしまう。安定版が欲しい場合は、リリース一覧から
`Nightly`ではない最初のリリースを明示的に選ぶこと:
```powershell
$release = Invoke-RestMethod -Uri "https://api.github.com/repos/simutrans/simutrans/releases" |
    Where-Object { $_.tag_name -ne "Nightly" } | Select-Object -First 1
```
（ビルド不要。公式が既にWindows向けビルド済みzipを配布している）
```powershell
# アセット名の"60-11"部分（makeobjのバージョン番号）はリリースごとに変わりうるため、
# $release.assets から実際のファイル名（"makeobj-win-*.zip"パターン）を動的に拾う。
# 固定文字列として決め打ちしないこと。
$asset = $release.assets | Where-Object { $_.name -like "makeobj-win-*.zip" } | Select-Object -First 1
$url = $asset.browser_download_url
$dest = "$env:LOCALAPPDATA\Programs\makeobj"
New-Item -ItemType Directory -Force -Path $dest | Out-Null
Invoke-WebRequest -Uri $url -OutFile "$env:TEMP\makeobj-win.zip"
Expand-Archive -Path "$env:TEMP\makeobj-win.zip" -DestinationPath $dest -Force
# zipの中身はmakeobj.exe単体（サブフォルダ無し）を直接展開先に配置する構成
```
PATH追加は3-1と同じ要領（`$dest`を`makeobj`用のパスに読み替える）。

### 3-3. 旧拡張アンインストール
```powershell
code --uninstall-extension 128na.simutrans-dat-vscode-extention
```

### 3-4. 新拡張インストール
```powershell
code --install-extension 128na.simutrans-dat-linter
```
dat_linterがPATHに通っていれば、`simutransDatLinter.executablePath`は
デフォルト値（`"dat_linter"`）のままで動くため追加設定は不要。PATHに通っていない
場合のみ、ユーザーのVSCode設定（`settings.json`、User設定 or ワークスペース設定）に
絶対パスを書き込む:
```json
{
  "simutransDatLinter.executablePath": "C:\\Users\\<user>\\AppData\\Local\\Programs\\dat_linter\\dat_linter.exe"
}
```

## Step 4: 結果報告

実行した項目・スキップした項目・**PATH変更を反映するには新しいターミナル/VSCode
ウィンドウを開き直す必要がある旨**をまとめて報告する。可能であれば新しいシェルで
`dat_linter -v`/`makeobj`（バージョン確認）を実行し、実際に反映されたことを確認する。
