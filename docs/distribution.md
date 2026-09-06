# 配布と資源計測

## source package

`segzify` はRust crateとして配布できるよう、`Cargo.toml` の `include` で
實行に必要なsource、埋込み辭書、文書、licenseを明示している。fontのように
CLIの實行に不要な大型資源はsource packageへ含めない。

release前にはclean working treeで次を實行する。

```sh
cargo package
```

これはpackageに含まれるfilesを確認し、packageされたcrateをbuildして検證する。
公開前にcrate registryのnameが利用可能であることも確認すること。

registryへ公開するまでは、利用者はREADMEに示すGit repositoryまたはsource checkoutから
`cargo install` する。platform別のprebuilt binaryや自動publishは、このrepositoryでは
まだ提供しない。

## 實行檔容量

`segzify` はUniDicと變換dataを埋込むため、release版實行檔容量を配布資源の基準値とする。
次のcommandはlocked dependenciesでrelease buildを作成し、論理容量をbyte單位で表示する。

```sh
sh scripts/measure-release.sh
```

測定値はtarget platform、Rust compiler、linkerにより變動する。release候補を比較する時は、
同じhost・toolchainで測定し、commandの出力をrelease noteへ記錄する。
