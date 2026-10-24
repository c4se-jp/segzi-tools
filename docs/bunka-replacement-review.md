# 文化廳置換表の保守

`dic/compound_replacements_bunka.tsv` は、文化廳「同音の漢字による書きかえ」に由來する置換表である。全行の檢討は完了してをり、`kind` は作業の進捗ではなく、變換時の扱ひを示す。

## 分類

- `safe`: 語義・用法を變へない自動置換として採用した規則。UniDicの語境界で保護して適用する。
- `candidate`: 檢討の結果、一律の自動置換には採用せず、利用者の確認用に殘した候補。語義・讀み・專門用語・固有名等による制約や、指定表記の同等性を確認できなかつた理由を `note` に記錄する。

`candidate` は檢討待ちではなく、`safe` へ移す必要もない。入力中で語境界が一致すれば、自動置換せず `unresolved_bunka_replacements` に報吿する。ここで未解決なのは辭書の檢討ではなく、その入力文脈での採否である。`--fail-on-unresolved` の對象にもなる。

舊分類 `pending` は廢止し、旣存の該當行は判定理由・出典を保つて `candidate` に統一した。變換器と監査scriptは `safe` と `candidate` のみを受附ける。

## 整合性の監査

`make bunka-audit` 又は `bb scripts/audit_bunka_replacements.clj` を實行する。`candidate` を變更字對ごとに件數順で束ねた一覽は、分類の把握・保守のためのものであり、未檢討項目の一覽ではない。

scriptは、欄數及び `kind`、重複するsource、通常の熟語規則と重なる `safe`、`safe` 同士の置換連鎖を檢査する。不正な行又は衝突があれば終了statusは非零となる。

## 規則を追加・變更する場合

1. 文化廳原表、辭書、用例を調べ、語義同一性、正字化規則との整合、專門語としての用法を確認する。
2. 一律の自動置換が妥當なら `safe`、文脈ごとの確認用に殘すなら `candidate` とする。候補としても不要なら行を削除する。判定理由と出典位置はTSVの `note` に殘す。旣存の `note` の「保留」は、自動置換を採用しなかつた理由として讀む。
3. `safe` の置換、語中での見送り、`candidate` の報吿を代表例でtestする。
4. TSVを變更したら `dic/MANIFEST.json` のSHA-256を更新し、`cargo test`、`make bunka-audit`、`bb scripts/verify_dic_manifest.clj` を實行する。

一つのpull requestでは、一つの字對又は意味的に近い少數の字對だけを變更する。分類名のみの移行では、個々の語の判定を變へない。
