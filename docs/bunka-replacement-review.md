# 文化廳置換候補の檢討

`dic/compound_replacements_bunka.tsv` の `pending` は、文化廳「同音の漢字による書きかえ」に由來する候補である。語義又は用法が同一であることを確認できるまで、自動置換してはならない。

## 手順

1. `bb scripts/audit_bunka_replacements.clj` を實行する。出力は、`pending` を變更字對ごとに件數順で束ねた一覽である。
2. 一つの字對について、文化廳原表、辭書、用例を調べる。語義同一性、正字化規則との整合、專門語としての用法を確認する。
3. 置換を採用するなら `safe` にする。自動置換できなければ `pending` に維持する。警吿候補にも不要なら行を削除する。`candidate` は現狀で `pending` と同じく未解決として報吿されるので、判定結果には使はない。
4. `safe` の置換、語中での見送り、`pending` の報吿を代表例でtestする。`cargo test` と `bb scripts/verify_dic_manifest.clj` を實行する。

scriptは、欄數及び `kind`、重複するsource、通常の熟語規則と重なる `safe`、`safe` 同士の置換連鎖を檢査する。衝突があればstatus 1で終了する。

一つのpull requestでは、一つの字對又は意味的に近い少數の字對だけを判定する。判定理由と原表の出典位置はTSVの `note` に殘す。
