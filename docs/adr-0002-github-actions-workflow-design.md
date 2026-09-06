# ADR 0002: GitHub Actions workflowの構成を規約化する

- Status: Accepted
- Date: 2026-09-07

## Context

Pull RequestでRust crateとGitHub Actions設定を再現可能に検証しつつ、hookの入口と共通処理の所在を明確にする必要がある。

## Decision

- hookごとに`on-HOOKNAME.yaml`を1つ作り、原則としてjobをここに置く。
- 長大なjob、または複数hookで共有するjobは、`wf-WORKFLOWNAME.yaml`の`workflow_call`へ分離する。
- `on-pull_request.yaml`は、`Cargo.toml`の`rust-version`を`cargo metadata`から取得して`cargo test --locked`を実行する。
- 同workflowから`wf-lint-gha.yaml`を呼び出し、`ne-sachirou/lint-gha-reviewdog`でGitHub Actionsを検証する。必要な書込み権限はlint jobに限定する。
- 外部Actionはcommit SHAで固定し、tagとの対応コメントを付ける。固定・更新には`pinact run`を使う。
- workflowは`yamllint .github/workflows/`、`actionlint`、`zizmor .`、`ghalint run`を通過させる。

## Consequences

- hookと共通workflowの責務がfile名から分かり、共通処理を重複させない。
- Rust版はmanifestだけで管理され、CIとの不一致を防ぐ。
- Action更新は意図的な`pinact`実行を要するが、参照するAction revisionを監査できる。

## Alternatives considered

- **全jobを各`on-*.yaml`へ複製する:** 共通lintの変更漏れを招くため採用しない。
- **Action tagをそのまま参照する:** 実行内容が後から変わるため採用しない。
- **CIにRust版を直接記述する:** `Cargo.toml`と二重管理になるため採用しない。
