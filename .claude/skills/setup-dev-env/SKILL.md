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
(Get-Command dat_linter -ErrorAction SilentlyContinue).Source   # 実体パス（下記参照）
```
最新版と比較する場合:
```powershell
(Invoke-RestMethod -Uri "https://api.github.com/repos/128na/simutrans-dat-linter/releases/latest").tag_name
```

**バージョンだけでなく実体パスも必ず確認すること。** `-v`が返るだけでは「導入済み」と
早合点しない。実体パス（`.Source`）がこのスキルの標準導入先
（`$env:LOCALAPPDATA\Programs\dat_linter\dat_linter.exe`）と異なる場合、
「導入済みだが場所が想定と違う」ケースであり、そのままStep 3で標準導入先へ
新規導入すると**2つの実体がPATH上に並存**し、どちらが有効かユーザーが把握できなくなる。
このケースはStep 2の選択肢に含めて事前にユーザーへ確認すること（Step 3実行中に
初めて気づいて場当たり的に聞き返す、という流れにしない）。

### 1-2. makeobj
```powershell
Get-Command makeobj -ErrorAction SilentlyContinue
makeobj   # 引数無しで実行すると "Makeobj version 60.11 for Simutrans 124.5 and higher" のような
          # バージョン行が出力される（エラー終了するが、バージョン確認には十分）
(Get-Command makeobj -ErrorAction SilentlyContinue).Source   # 実体パス（下記参照）
```
最新版と比較する場合は下記「3-2の罠」を参照（`/releases/latest` はそのまま使えない）。

**dat_linterと同様、実体パス（`.Source`）も必ず確認すること。** このスキルの標準導入先
（`$env:LOCALAPPDATA\Programs\makeobj\makeobj.exe`）と異なる場所（例:
`C:\bin\makeobj.exe`のような手動導入・scoop/choco経由の導入）に既にある場合、
「導入済みだが場所が想定と違う」ケースとして扱う。この場合にStep 3をそのまま実行すると、
標準導入先とは別の実体を上書きしてしまう（今回の実機検証で実際に発生した）か、
2つの実体がPATH上に並存する事態になる。Step 2の選択肢に含めて事前に確認すること。

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

**実体パスが標準導入先と異なる場合（dat_linter/makeobj）**は、「導入済み」として単純に
スキップ扱いにせず、以下のような選択肢を明示的に提示すること（Step 3の実行中に
初めて発覚して場当たり的に確認する、という流れを避けるため）:
- 現状の実体をそのまま使う（何もしない。PATH上のその実体が今後も有効であり続ける）
- 標準導入先（`$env:LOCALAPPDATA\Programs\...`）へ新たに導入し直す（既存の実体は
  そのまま残るため、PATH上で2つの実体が並存しうる旨を明記した上で確認する）

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
