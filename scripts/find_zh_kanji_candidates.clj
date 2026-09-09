#!/usr/bin/env bb
(ns find-zh-kanji-candidates
  "简化字・繁體字から日本語漢字への變換候補を抽出する。"
  (:require
    [babashka.cli :as cli]
    [babashka.fs :as fs]
    [babashka.http-client :as http]
    [cheshire.core :as json]
    [clojure.string :as str])
  (:import
    (java.io
      ByteArrayInputStream)
    (java.nio
      ByteBuffer)
    (java.nio.charset
      CodingErrorAction
      StandardCharsets)
    (java.util.zip
      ZipInputStream)))


(def unihan-zip-url "https://www.unicode.org/Public/UCD/latest/ucd/Unihan.zip")


(def data-dir
  (-> *file*
      fs/path
      fs/parent
      fs/parent
      (fs/path "dic")))


(defn parse-args
  []
  (let [{:keys [opts args]}
        (cli/parse-args
          *command-line-args*
          {:coerce {:data-dir fs/path
                    :min-count parse-long
                    :output fs/path
                    :scan-dir [fs/path]
                    :unihan-variants fs/path}
           :spec {:data-dir {:default data-dir}
                  :min-count {:default 1}
                  :scan-dir {:require true}}})]
    (when (seq args)
      (throw (ex-info "Unexpected positional arguments" {:args args})))
    opts))


(defn decode-utf8-ignoring-errors
  [bytes]
  (let [decoder (doto (.newDecoder StandardCharsets/UTF_8)
                  (.onMalformedInput CodingErrorAction/IGNORE)
                  (.onUnmappableCharacter CodingErrorAction/IGNORE))]
    (str (.decode decoder (ByteBuffer/wrap bytes)))))


(defn fetch-unihan-variants
  []
  (let [{:keys [body status]} (http/get unihan-zip-url {:as :bytes :throw false})]
    (when-not (= status 200)
      (throw (ex-info "Unable to fetch Unihan ZIP" {:status status})))
    (with-open [zip (ZipInputStream. (ByteArrayInputStream. body))]
      (loop [entry (.getNextEntry zip)]
        (cond
          (nil? entry) (throw (ex-info "Unihan_Variants.txt is missing from ZIP" {}))
          (= "Unihan_Variants.txt" (.getName entry))
          (decode-utf8-ignoring-errors (.readAllBytes zip))
          :else (recur (.getNextEntry zip)))))))


(defn to-char
  [codepoint]
  (String. (Character/toChars (Integer/parseInt (subs codepoint 2) 16))))


(defn load-variant-pairs
  [variants-text]
  (reduce (fn [pairs line]
            (let [parts (str/split line #"\t")]
              (if (or (str/blank? line)
                      (str/starts-with? line "#")
                      (< (count parts) 3)
                      (not= "kTraditionalVariant" (nth parts 1)))
                pairs
                (let [source (to-char (nth parts 0))]
                  (reduce (fn [pairs token]
                            (let [code (first (str/split token #"<"))]
                              (if (str/starts-with? code "U+")
                                (update pairs source (fnil conj #{}) (to-char code))
                                pairs)))
                          pairs
                          (str/split (nth parts 2) #"\s+"))))))
          {}
          (str/split-lines variants-text)))


(defn load-known-sources
  [data-dir]
  (let [kiuzi (json/parse-string (slurp (str (fs/path data-dir "kiuzi_map.json"))))
        known (into #{} (keys (get kiuzi "char_map")))]
    (into known
          (concat (vals (get kiuzi "char_map"))
                  (keys (get kiuzi "ambiguous_characters"))
                  (mapcat (fn [name]
                            (let [path (fs/path data-dir name)]
                              (if-not (fs/exists? path)
                                []
                                (for [line (str/split-lines (slurp (str path)))
                                      :let [line (str/trim line)]
                                      :when (and (not (str/blank? line))
                                                 (not (str/starts-with? line "#")))]
                                  (first (str/split line #"\t"))))))
                          ["zh_char_map.tsv" "zh_compound_map.tsv"])
                  (let [path (fs/path data-dir "zh_ambiguous_characters.json")]
                    (if (fs/exists? path)
                      (keys (json/parse-string (slurp (str path))))
                      []))))))


(defn load-compound-sources
  [data-dir]
  (let [path (fs/path data-dir "zh_compound_map.tsv")]
    (if-not (fs/exists? path)
      #{}
      (into #{}
            (for [line (str/split-lines (slurp (str path)))
                  :let [line (str/trim line)
                        source (first (str/split line #"\t"))]
                  :when (and (not (str/blank? line))
                             (not (str/starts-with? line "#"))
                             (not (str/blank? source)))]
              source)))))


(defn code-point-compare
  [left right]
  (let [left-code-points (.toArray (.codePoints ^String left))
        right-code-points (.toArray (.codePoints ^String right))
        limit (min (alength left-code-points) (alength right-code-points))]
    (loop [index 0]
      (if (= index limit)
        (Long/compare (alength left-code-points) (alength right-code-points))
        (let [order (Long/compare (aget left-code-points index)
                                  (aget right-code-points index))]
          (if (zero? order)
            (recur (inc index))
            order))))))


(defn resolve-ja-candidates
  [char variant-pairs reverse-kiuzi]
  (into #{}
        (for [partner (get variant-pairs char #{})
              :when (not= partner char)]
          (get reverse-kiuzi partner partner))))


(defn mark-covered
  [text compound-sources]
  (let [covered (boolean-array (.length text))]
    (doseq [compound compound-sources]
      (loop [start 0]
        (let [index (.indexOf text compound start)]
          (when-not (= -1 index)
            (dotimes [offset (.length compound)]
              (aset-boolean covered (+ index offset) true))
            (recur (+ index (.length compound)))))))
    covered))


(defn count-text-characters
  [state text targets compound-sources]
  (let [covered (mark-covered text compound-sources)]
    (loop [index 0
           state state]
      (if (>= index (.length text))
        state
        (let [code-point (.codePointAt text index)
              char-count (Character/charCount code-point)
              char (String. (Character/toChars code-point))]
          (recur (+ index char-count)
                 (if (or (aget covered index) (not (contains? targets char)))
                   state
                   (let [count (get-in state [:counts char] 0)]
                     (cond-> (update-in state [:counts char] (fnil inc 0))
                       (zero? count)
                       (-> (assoc-in [:first-index char] (:next-index state))
                           (update :next-index inc)))))))))))


(defn count-corpus-characters
  [scan-dirs targets compound-sources]
  (reduce (fn [state scan-dir]
            (reduce (fn [state path]
                      (count-text-characters state
                                             (decode-utf8-ignoring-errors
                                               (fs/read-all-bytes path))
                                             targets
                                             compound-sources))
                    state
                    (concat (fs/glob scan-dir "*.md")
                            (fs/glob scan-dir "**/*.md"))))
          {:counts {} :first-index {} :next-index 0}
          scan-dirs))


(defn ordered-counts
  [{:keys [counts first-index]}]
  (sort (fn [[char-a count-a] [char-b count-b]]
          (let [count-order (compare count-b count-a)]
            (if (zero? count-order)
              (compare (get first-index char-a) (get first-index char-b))
              count-order)))
        counts))


(defn main
  []
  (let [{:keys [data-dir min-count output scan-dir unihan-variants]} (parse-args)
        variants-text (if unihan-variants
                        (decode-utf8-ignoring-errors
                          (fs/read-all-bytes unihan-variants))
                        (fetch-unihan-variants))
        variant-pairs (load-variant-pairs variants-text)
        kiuzi (json/parse-string (slurp (str (fs/path data-dir "kiuzi_map.json"))))
        reverse-kiuzi (reduce (fn [reversed [shinjitai seiji]]
                                (if (contains? reversed seiji)
                                  reversed
                                  (assoc reversed seiji shinjitai)))
                              {}
                              (sort (fn [[left _] [right _]]
                                      (code-point-compare left right))
                                    (get kiuzi "char_map")))
        known-sources (load-known-sources data-dir)
        compound-sources (load-compound-sources data-dir)
        unknown-variant-chars (reduce disj (set (keys variant-pairs)) known-sources)
        counts (count-corpus-characters scan-dir
                                        unknown-variant-chars
                                        compound-sources)
        rows (for [[char count] (ordered-counts counts)
                   :when (>= count min-count)
                   :let [candidates (disj (resolve-ja-candidates char
                                                                 variant-pairs
                                                                 reverse-kiuzi)
                                          char)]
                   :when (seq candidates)]
               (str char "\t" count "\t"
                    (str/join "|" (sort code-point-compare candidates))))
        output-text (str "# char\tcount\tja_candidates(要人手選別)\n"
                         (str/join "\n" rows)
                         "\n")]
    (if output
      (do
        (spit (str output) output-text)
        (println (format "wrote %s (%d candidates)" output (count rows))))
      (print output-text))
    0))


(System/exit (main))
