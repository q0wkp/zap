<div align="center">

<img src="assets/zap-logo.svg" alt="Zap" width="128" />

# Zap

[English](./README.md) · [简体中文](./README.zh-CN.md)

<sub><i>現在は <a href="https://github.com/warpdotdev/warp">Warp</a> をベースにしていますが、今後は独自に進化していきます。</i></sub>

</div>

Zap はオープンでローカルファーストなターミナルで、AI と Agent をファーストクラスでサポートします。任意の AI プロバイダーを接続し、任意の CLI Agent を取り込み、ターミナル内で SSH ホストを管理 —— API キー・履歴・Agent の状態はデフォルトで自分のマシンに留まります。

## 公式 Warp に対して Zap が追加する機能

- **クラウド必須なし** —— アカウント、ログイン、Drive 同期、クラウド Agent 履歴のいずれも不要。
- **BYOP な AI プロバイダー** —— 任意の OpenAI 互換エンドポイントに加え、OpenAI / Anthropic / Gemini / DeepSeek / Ollama のネイティブプロトコル。API キーはローカルに保持。
- **サードパーティ CLI Agent** —— DeepSeek-TUI / Codex CLI / Claude Code / Google Antigravity (`agy`) を Block と通知センターに統合。
- **内蔵 SSH ホストマネージャー** —— ターミナル内でホスト・設定・セッションを管理、tmux・zmx と連携。
- **編集可能なシステムプロンプト** —— minijinja テンプレートをクライアント側でレンダリング。
- **レンダリング改善** —— Markdown パイプラインのチューニング、CJK ソフトラップ caret と太字サブピクセルの修正。
- **多言語 UI** —— 英語 / 簡体字中国語 / 日本語をデフォルト同梱、コミュニティで拡張可能。
- **プライバシー優先のデフォルト** —— Cloud Agent / Computer Use / Referral / テレメトリはデフォルトでオフ。

## OpenWarp / Warp からの移行

プロジェクトが Zap に改名される前から使っていた方(当時の名称は **OpenWarp**)、
または上流 **Warp** から乗り換える方は、
[docs/migrate-from-warp.ja.md](docs/migrate-from-warp.ja.md) を参照して設定を
引き継いでください。

## フォークのリリース

このフォークは独自のリリースを公開し、自動更新もアップストリームではなくこちらを確認します。タグはアップストリームの
次のバージョンに `-mod.N` を付けた形式で、`v2026.07.09.1` なら `v2026.07.09.2-mod.1` になります。`v*` タグをプッシュ
すると `.github/workflows/release.yml` が実行されます。
各プラットフォームはビルドが終わり次第 prerelease にアップロードされ、すべて揃うと latest になります。自動更新が見るのは latest だけです。

## ロードマップ

[docs/roadmap.ja.md](docs/roadmap.ja.md) を参照してください。

## 謝辞

- [Warp](https://github.com/warpdotdev/warp) —— Zap がベースとしている上流のターミナル。
- [DeepSeek-TUI](https://github.com/Hmbown/DeepSeek-TUI) —— 深く統合された CLI Agent パートナー。
