#!/usr/bin/env bb
(ns audit-bunka-replacements
  "文化廳置換候補を置換字對ごとに表示し、辭書の衝突を檢査する。"
  (:require
    [babashka.fs :as fs]
    [clojure.string :as string]))


(def data-dir
  (-> *file*
      fs/path
      fs/parent
      fs/parent
      (fs/path "dic")))


(def bunka-path (fs/path data-dir "compound_replacements_bunka.tsv"))
(def ordinary-path (fs/path data-dir "compound_replacements.tsv"))
(def allowed-kinds #{"safe" "candidate" "pending"})


(defn rows
  [path]
  (->> (string/split-lines (slurp (str path)))
       (remove #(or (string/blank? %) (string/starts-with? % "#")))
       (mapv #(string/split % #"\t" -1))))


(defn bunka-rows
  []
  (mapv
    (fn [line-number row]
      (when (not= 4 (count row))
        (throw (ex-info "文化廳置換表の欄數が不正です"
                        {:line-number line-number :row row})))
      (let [[source target kind note] row]
        (when-not (allowed-kinds kind)
          (throw (ex-info "文化廳置換表のkindが不正です"
                          {:line-number line-number :kind kind})))
        {:source source :target target :kind kind :note note}))
    (range 2 Long/MAX_VALUE)
    (rows bunka-path)))


(defn changed-characters
  [{:keys [source target]}]
  (let [source-characters (vec source)
        target-characters (vec target)]
    (when (not= (count source-characters) (count target-characters))
      (throw (ex-info "sourceとtargetの字數が異なります"
                      {:source source :target target})))
    (->> (map vector source-characters target-characters)
         (remove (fn [[from to]] (= from to)))
         (map (fn [[from to]] (str from "→" to)))
         (string/join "、"))))


(defn duplicate-sources
  [replacements]
  (->> replacements
       (group-by :source)
       (keep (fn [[source rows]]
               (when (< 1 (count rows))
                 [source rows])))))


(defn ordinary-sources
  []
  (into #{}
        (map first)
        (rows ordinary-path)))


(defn report-conflicts
  [replacements]
  (let [safe-replacements (filter #(= "safe" (:kind %)) replacements)
        duplicates (duplicate-sources replacements)
        ordinary-overlaps (filter #(contains? (ordinary-sources) (:source %))
                                  safe-replacements)
        safe-sources (into #{} (map :source) safe-replacements)
        chains (filter #(contains? safe-sources (:target %)) safe-replacements)]
    (doseq [[source rows] duplicates]
      (binding [*out* *err*]
        (println (format "重複source: %s (%d件)" source (count rows)))))
    (doseq [{:keys [source target]} ordinary-overlaps]
      (binding [*out* *err*]
        (println (format "通常規則と衝突: %s -> %s" source target))))
    (doseq [{:keys [source target]} chains]
      (binding [*out* *err*]
        (println (format "safe置換連鎖: %s -> %s" source target))))
    (empty? (concat duplicates ordinary-overlaps chains))))


(defn print-pending-groups
  [replacements]
  (doseq [[change rows] (->> replacements
                             (filter #(= "pending" (:kind %)))
                             (group-by changed-characters)
                             (sort-by (fn [[change rows]]
                                        [(- (count rows)) change])))]
    (println (format "%d\t%s" (count rows) change))
    (doseq [{:keys [source target]} (sort-by (juxt :source :target) rows)]
      (println (format "\t%s\t%s" source target)))))


(defn main
  []
  (let [replacements (bunka-rows)]
    (print-pending-groups replacements)
    (if (report-conflicts replacements) 0 1)))


(System/exit (main))
