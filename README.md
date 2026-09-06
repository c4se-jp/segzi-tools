# 正字關聯道具群 (seiji-tools)

正字 (舊字體とも呼ばれる) で文を書く助けになる道具群。

[その環境で利用出來る範囲内で出來る丈正字にする方法 - 兩河世界](https://scrapbox.io/yuraru/%E3%81%9D%E3%81%AE%E7%92%B0%E5%A2%83%E3%81%A7%E5%88%A9%E7%94%A8%E5%87%BA%E4%BE%86%E3%82%8B%E7%AF%84%E5%9B%B2%E5%86%85%E3%81%A7%E5%87%BA%E4%BE%86%E3%82%8B%E4%B8%88%E6%AD%A3%E5%AD%97%E3%81%AB%E3%81%99%E3%82%8B%E6%96%B9%E6%B3%95)

## segzify

`segzify` は日本語の文章を正字・正かなづかひへ變換するcommand-line toolです。
辭書は實行檔に埋込まれるため、導入後に別途の辭書dataは要りません。

### 導入

Rust 1.98.1以降を用意して、Git repositoryからinstallできます。

```sh
cargo install --git https://github.com/c4se-jp/segzi-tools.git --locked
```

cloneしたworking treeからinstallする場合:

```sh
cargo install --path . --locked
```

source packageに含めるfilesとrelease版實行檔の容量を測定する方法は
[配布と資源計測](docs/distribution.md)を參照してください。

### 使用法

標準入力から讀み、變換結果を標準出力へ書きます。reportは標準errorへ書きます。

```sh
printf '国語を学ぶ\n' | segzify
```

input/output fileを指定する例:

```sh
segzify original.txt --output converted.txt --report json --report-output report.json
```

既存fileがすでに變換濟みかを調べるには `--check` を使います。差分があれば
終了status 1となり、標準出力には書きません。

```sh
segzify --check original.txt
```

曖昧な字または語境界のため見送った熟語置換をerrorにするには、
`--fail-on-unresolved` を追加します。この場合の終了statusは 2です。
`--check` と同時ならstatusはbitwise ORされます。usage errorは64、input讀取失敗は66、
初期化失敗は70、output書込失敗は74です。

## Font

[font/](font/)

## CONTRIBUTING

[CONTRIBUTING](.github/CONTRIBUTING.md)
