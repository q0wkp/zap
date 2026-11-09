# Zap への設定の移行

[English](./migrate-from-warp.md) · [简体中文](./migrate-from-warp.zh-CN.md)

このガイドは、**設定系の構成**(カスタムキーバインド・テーマ・ワークフロー・
MCP 設定など)を以前のインストールから Zap へ引き継ぎたい方向けです。

移行元として想定されるのは 2 つあり、**両者では安全性プロファイルが異なる**ため、
本書では別々のセクションで扱います。両方該当する場合は、**まず OpenWarp から
移行してから** Warp の移行を検討してください。

1. **OpenWarp** —— Zap の以前の名称。
2. **上流の [Warp](https://github.com/warpdotdev/warp)** —— Zap が fork して
   いるプロジェクト。

本書は コマンド履歴・SQLite データベース・Drive オブジェクト・認証情報の
いずれも **意図的に対象外** としています。これらはマシン依存のストア
(Keychain / libsecret)、または schema が結合したストアに格納されて
いるため、コピーは安全ではありません。

---

## ディスク上のレイアウト

Zap(および OpenWarp / 上流 Warp も同様)はディスク上の状態を **3 種類の
ディレクトリ**に分けて格納します:

- **config** —— `settings.toml`、`keybindings.yaml`
- **data** —— `themes/`、`workflows/`、`launch_configurations/`、`tab_configs/`
- **home dotfile** —— `.mcp.json`、`skills/`

macOS では 3 カテゴリーがいずれも単一の home dotfile ディレクトリ
(`~/.warp/`、`~/.openwarp/`、または `~/.zap/`)に集約されます。Linux では
XDG 規約に従って **3 つの異なる場所** に分かれます。下記の移行スクリプトは
プラットフォームごとに各ファイルを正しい保存先に配置します。

### Zap の保存先

| カテゴリー | macOS | Linux |
|---|---|---|
| config | `~/.zap/` | `${XDG_CONFIG_HOME:-~/.config}/zap/` |
| data | `~/.zap/` | `${XDG_DATA_HOME:-~/.local/share}/zap/` |
| home dotfile | `~/.zap/` | `~/.zap/` |

### OpenWarp のソースパス

| カテゴリー | macOS | Linux |
|---|---|---|
| config | `~/.openwarp/` | `${XDG_CONFIG_HOME:-~/.config}/openwarp/` |
| data | `~/.openwarp/` | `${XDG_DATA_HOME:-~/.local/share}/openwarp/` |
| home dotfile | `~/.openwarp/` | `~/.openwarp/` |

### 上流 Warp のソースパス

| カテゴリー | macOS | Linux |
|---|---|---|
| config | `~/.warp/` | `${XDG_CONFIG_HOME:-~/.config}/warp-terminal/` |
| data | `~/.warp/` | `${XDG_DATA_HOME:-~/.local/share}/warp-terminal/` |
| home dotfile | `~/.warp/` | `~/.warp/` |

> Linux 上のディレクトリ名 `warp-terminal` は Linux パッケージ名と一致します
> (Debian/Ubuntu では `/opt/warpdotdev/warp-terminal/` など)。

---

## 1. OpenWarp からの移行(既存ユーザーの推奨経路)

OpenWarp は改名前の Zap そのものです。改名コミット
(`feat: rename project Warp/OpenWarp → Zap`)は識別子とディスク上のパス名を
変更しただけで、**設定ファイルのフォーマットと schema は一切変わっていません**。
下記のファイルはそのままコピーできます。

### コピー対象

| ファイル / ディレクトリ | カテゴリー | 役割 |
|---|---|---|
| `settings.toml` | config | 公開設定(TOML ベースの設定ファイル)。 |
| `keybindings.yaml` | config | カスタムキーバインド。 |
| `themes/` | data | カスタムテーマ。 |
| `workflows/` | data | カスタムワークフロー。 |
| `launch_configurations/` | data | Launch 設定。 |
| `tab_configs/` | data | Tab 設定。 |
| `.mcp.json` | home dotfile | MCP サーバー設定。 |
| `skills/` | home dotfile | Agent skills。 |

### 手順

> コピー前に **Zap を終了**してください。プロセスがファイルを掴んでいると
> 失敗します。

**macOS**

```sh
mkdir -p "$HOME/.zap"
for f in settings.toml keybindings.yaml themes workflows launch_configurations tab_configs skills .mcp.json; do
  if [ -e "$HOME/.openwarp/$f" ] && [ ! -e "$HOME/.zap/$f" ]; then
    cp -R "$HOME/.openwarp/$f" "$HOME/.zap/$f"
  fi
done
```

**Linux**

```sh
src_config="${XDG_CONFIG_HOME:-$HOME/.config}/openwarp"
src_data="${XDG_DATA_HOME:-$HOME/.local/share}/openwarp"
src_home="$HOME/.openwarp"

dst_config="${XDG_CONFIG_HOME:-$HOME/.config}/zap"
dst_data="${XDG_DATA_HOME:-$HOME/.local/share}/zap"
dst_home="$HOME/.zap"
mkdir -p "$dst_config" "$dst_data" "$dst_home"

copy() {
  if [ -e "$1/$3" ] && [ ! -e "$2/$3" ]; then
    cp -R "$1/$3" "$2/$3"
  fi
}

copy "$src_config" "$dst_config" settings.toml
copy "$src_config" "$dst_config" keybindings.yaml
copy "$src_data"   "$dst_data"   themes
copy "$src_data"   "$dst_data"   workflows
copy "$src_data"   "$dst_data"   launch_configurations
copy "$src_data"   "$dst_data"   tab_configs
copy "$src_home"   "$dst_home"   .mcp.json
copy "$src_home"   "$dst_home"   skills
```

`[ ! -e ... ]` / `-not (Test-Path $to)` のガードは、Zap 側で既にカスタマイズ
した内容を上書きしないためのものです。OpenWarp の値で上書きしたい場合は外して
ください。

Zap が問題なく動くことを確認したら、上記の OpenWarp ディレクトリを削除して
ディスク領域を回収できます。もう誰も使っていません。

---

## 2. 上流 Warp からの移行

上流 Warp は独立した別プロダクトで、独自のディスク上アイデンティティを持ち
ます(上の「上流 Warp のソースパス」表を参照)。Zap はビルド時の channel が
`Oss` で、独自の app id(`dev.zap.Zap`)と、プラットフォームごとに分かれた
レイアウトを持ちます。両者のインストールは互いに相手のファイルを見ることが
できません —— これは Warp のアカウントやクラウド状態を Zap に持ち込まない
ための仕様でもあります。

下記のテキスト形式ファイルは schema が安定しており、コピーしても安全です。
**それ以外はそうではありません** —— Warp は Zap とは独立に進化しており、
バイナリ / プライベートストアは Warp の認証や bundle 識別子に紐付いている
場合があります。

### コピー対象

セクション 1 と同じ 8 項目です:

| ファイル / ディレクトリ | カテゴリー | 役割 |
|---|---|---|
| `settings.toml` | config | 公開設定(TOML ベースの設定ファイル)。 |
| `keybindings.yaml` | config | カスタムキーバインド。 |
| `themes/` | data | カスタムテーマ。 |
| `workflows/` | data | カスタムワークフロー。 |
| `launch_configurations/` | data | Launch 設定。 |
| `tab_configs/` | data | Tab 設定。 |
| `.mcp.json` | home dotfile | MCP サーバー設定。 |
| `skills/` | home dotfile | Agent skills。 |

### コピー**してはいけない**もの

- **`user_preferences.json`** —— macOS の
  `~/Library/Application Support/dev.warp.Warp/`(Linux ではそれに
  相当する state ディレクトリ)にあるプライベートストアで、ユーザー設定・
  認証トークン・マシン依存 ID・クラウドキャッシュが混在しています。ファイル
  ごとコピーすると ID 情報が漏れたり、Zap の認証状態が壊れたりします。
  Zap のデフォルトはもともとプライバシー優先なので、**触らないでください**。
- **`warp.sqlite`**(および `-wal` / `-shm` のサイドカー)—— schema が上流
  Warp に結合しており、Zap の migration が通る保証はありません。
- **Keychain / libsecret のエントリ** —— Warp の bundle / service 名に
  紐付いており、Zap からは利用できません。

### 手順

> コピー前に **Warp と Zap を終了**してください。

**macOS**

```sh
mkdir -p "$HOME/.zap"
for f in settings.toml keybindings.yaml themes workflows launch_configurations tab_configs skills .mcp.json; do
  if [ -e "$HOME/.warp/$f" ] && [ ! -e "$HOME/.zap/$f" ]; then
    cp -R "$HOME/.warp/$f" "$HOME/.zap/$f"
  fi
done
```

**Linux**

```sh
src_config="${XDG_CONFIG_HOME:-$HOME/.config}/warp-terminal"
src_data="${XDG_DATA_HOME:-$HOME/.local/share}/warp-terminal"
src_home="$HOME/.warp"

dst_config="${XDG_CONFIG_HOME:-$HOME/.config}/zap"
dst_data="${XDG_DATA_HOME:-$HOME/.local/share}/zap"
dst_home="$HOME/.zap"
mkdir -p "$dst_config" "$dst_data" "$dst_home"

copy() {
  if [ -e "$1/$3" ] && [ ! -e "$2/$3" ]; then
    cp -R "$1/$3" "$2/$3"
  fi
}

copy "$src_config" "$dst_config" settings.toml
copy "$src_config" "$dst_config" keybindings.yaml
copy "$src_data"   "$dst_data"   themes
copy "$src_data"   "$dst_data"   workflows
copy "$src_data"   "$dst_data"   launch_configurations
copy "$src_data"   "$dst_data"   tab_configs
copy "$src_home"   "$dst_home"   .mcp.json
copy "$src_home"   "$dst_home"   skills
```

Warp 自体のデータは一切手を加えません —— Warp 本体はそのまま使い続けられます。

---

## 確認方法

Zap を起動すると、テーマピッカーにカスタムテーマ、キーバインドエディタに
カスタムキーバインド、ワークフローランチャーにカスタムワークフローが見える
はずです。`settings.toml` に存在するキーは、設定 UI 上でも移行元の値が反映
されます。

うまく反映されない場合、原因は上記 8 ファイルのいずれかに含まれています ——
テキストエディタで開いて確認するか、削除して Zap のデフォルトに戻して
ください。

## ロールバック

本書の操作はいずれも破壊的ではありません。コピーしたファイルはどれも Zap が
起動時にデフォルトから再生成できるものです。すべてを元に戻すには:

```sh
# macOS
rm -rf ~/.zap
```

```sh
# Linux
rm -rf "${XDG_CONFIG_HOME:-$HOME/.config}/zap"
rm -rf "${XDG_DATA_HOME:-$HOME/.local/share}/zap"
rm -rf "$HOME/.zap"
```

OpenWarp および Warp のソースディレクトリは、いずれも本ガイドからは変更されません。
