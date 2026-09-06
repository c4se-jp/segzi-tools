# ADR 0001: segzifyをRust CLIとして移植する

- Status: Accepted
- Date: 2026-09-07

## Context

舊`segzify/`にはPython、Node.js、InDesign向けの複數實裝と個別の依存・辭書があり、同じ變換を再現・配布する境界が明確でなかった。正字・正かなづかひへの變換は、熟語、字、かな、patternの順序と、語境界・曖昧字の扱いを一貫して保つ必要がある。

## Decision

- `segzify`を單一のRust crate / CLIとして提供し、舊實裝・舊辭書を撤去する。
- 變換dataとUniDicを實行fileへ埋込み、外部dataの配置を導入時の前提にしない。
- `zh_compound_map.tsv`による中國語由來の熟語正規化は、境界を判定せず先行適用する。後續の熟語置換はUniDicの語境界で保護し、見送った箇所と未解決の曖昧字はreportする。
- CLIは標準入力/標準出力を旣定とし、reportは標準errorへ出す。`--check`、`--fail-on-unresolved`、明示した終了statusで自動處理できる契約を提供する。
- Cargo source packageの收錄filesをallowlistで固定し、CLIに不要なfontは含めない。Git/source checkoutからの`cargo install`を配布導線とし、release實行file容量はscriptで測定する。

## Consequences

- 利用者はRust toolchainだけで同一のCLIをbuild・installでき、辭書dataを別途管理しない。
- 埋込みUniDicによりrelease實行fileは大きくなるため、容量をrelease時の確認項目とする。
- reportを無視しても變換はできるが、嚴密な處理では`--fail-on-unresolved`を使う必要がある。
- crate registry公開とplatform別prebuilt binaryは、この決定の範圍外であり、必要になった時點で別ADRで扱ふ。

## Alternatives considered

- **舊實裝を倂存する:** 結果差・依存更新・配布手順を維持するコストが高いため採用しない。
- **辭書を外部downloadする:** 容量は減るが、offline利用と再現性を損なうため採用しない。
- **曖昧字・境界見送りを默って通す:** 自動處理の安全性を下げるため、reportとfailure optionを提供する。
