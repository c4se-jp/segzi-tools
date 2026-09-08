#!/usr/bin/env bb
(ns update-kiuzi-data
  "公開spreadsheetのCSVから正字變換用snapshot JSONを生成する。"
  (:require
    [babashka.cli :as cli]
    [babashka.fs :as fs]
    [babashka.http-client :as http]
    [cheshire.core :as json]
    [clojure.data.csv :as csv]
    [clojure.string :as str])
  (:import
    (java.io
      StringReader)
    (java.math
      BigInteger)
    (java.nio.file
      Files)
    (java.security
      MessageDigest)
    (java.time
      OffsetDateTime
      ZoneOffset)
    (java.time.format
      DateTimeFormatter)
    (java.time.temporal
      ChronoUnit)
    (java.util.regex
      Pattern)))


(def default-source-url
  (str "https://docs.google.com/spreadsheets/d/"
       "1CEBTf13rCCnA99Fvyg6PbBQTBRJN1JhevifxrfVGHE0/export?format=csv&gid=0"))


(def data-dir
  (-> *file*
      fs/path
      fs/parent
      fs/parent
      (fs/path "dic")))


(def default-output (fs/path data-dir "kiuzi_map.json"))
(def manifest-path (fs/path data-dir "MANIFEST.json"))


(def python-date-formatter
  (DateTimeFormatter/ofPattern "yyyy-MM-dd'T'HH:mm:ss.SSSSSSxxx"))


(def usage
  (str "Usage: bb scripts/update_kiuzi_data.clj [OPTIONS]\n"
       "\n"
       "Options:\n"
       "  --csv PATH          既に取得したCSVを使ふ\n"
       "  --source-url URL    CSVの取得元URL\n"
       "  --output PATH       snapshotの出力先\n"
       "  -h, --help          このhelpを表示する"))


(defn parse-args
  []
  (let [{:keys [opts args]}
        (cli/parse-args
          *command-line-args*
          {:spec {:csv {:coerce fs/path}
                  :help {:alias :h}
                  :source-url {:default default-source-url}
                  :output {:coerce fs/path :default default-output}}})]
    (when (seq args)
      (throw (ex-info "Unexpected positional arguments" {:args args})))
    opts))


(defn fetch-csv
  [url]
  (let [{:keys [body status]}
        (http/get url {:headers {"User-Agent" "Mozilla/5.0"}
                       :throw false
                       :timeout 30000})]
    (when-not (= status 200)
      (throw (ex-info "Unable to fetch CSV" {:status status :url url})))
    (str/replace-first body "\uFEFF" "")))


(defn resolve-target
  [row]
  (let [source (str/trim (get row "元字"))
        code (str/trim (get row "正碼"))
        ivs (str/trim (get row "正ivs"))
        compat (str/trim (get row "換字"))]
    (cond
      (or (str/blank? source) (str/blank? code)) nil
      (= code ".") (cond
                     (str/blank? ivs) nil
                     (not (str/blank? compat)) compat
                     :else source)
      :else (String. (Character/toChars (Integer/parseInt code 16))))))


(defn load-rows
  [csv-text]
  (let [[headers & rows] (csv/read-csv (StringReader. csv-text))]
    (map #(zipmap headers %) rows)))


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


(defn indentation
  [level]
  (apply str (repeat (* level 2) " ")))


(declare python-json)


(defn python-json-map
  [value level]
  (if (empty? value)
    "{}"
    (str "{\n"
         (str/join
           ",\n"
           (map (fn [key]
                  (str (indentation (inc level))
                       (json/generate-string key)
                       ": "
                       (python-json (get value key) (inc level))))
                (sort code-point-compare (keys value))))
         "\n"
         (indentation level)
         "}")))


(defn python-json-vector
  [value level]
  (if (empty? value)
    "[]"
    (str "[\n"
         (str/join
           ",\n"
           (map #(str (indentation (inc level))
                      (python-json % (inc level)))
                value))
         "\n"
         (indentation level)
         "]")))


(defn python-json
  [value level]
  (cond
    (map? value) (python-json-map value level)
    (sequential? value) (python-json-vector value level)
    (string? value) (json/generate-string value)
    (nil? value) "null"
    (true? value) "true"
    (false? value) "false"
    :else (str value)))


(defn sha256
  [path]
  (let [digest (MessageDigest/getInstance "SHA-256")]
    (format "%064x" (BigInteger. 1 (.digest digest (fs/read-all-bytes path))))))


(defn update-manifest
  [snapshot-path]
  (let [manifest (json/parse-string (slurp (str manifest-path)))
        file-name (str (fs/file-name snapshot-path))
        hash (sha256 snapshot-path)]
    (when-not (map? (get manifest "files"))
      (throw (ex-info "MANIFEST.json files must be an object" {})))
    (when-not (contains? (get manifest "files") file-name)
      (throw (ex-info "Snapshot is not listed in MANIFEST.json"
                      {:snapshot file-name})))
    (let [pattern (re-pattern
                    (str "(\"" (Pattern/quote file-name) "\"\\s*:\\s*)\"[0-9a-f]{64}\""))
          updated (str/replace-first (slurp (str manifest-path))
                                     pattern
                                     (str "$1\"" hash "\""))]
      (when (= updated (slurp (str manifest-path)))
        (throw (ex-info "Unable to update manifest hash" {:snapshot file-name})))
      (spit (str manifest-path) updated))))


(defn build-snapshot
  [rows source-url input-csv]
  (let [grouped
        (reduce (fn [groups row]
                  (let [source (str/trim (get row "元字"))]
                    (if (str/blank? source)
                      groups
                      (update groups source (fnil conj [])
                              {"target" (resolve-target row)
                               "row_id" (str/trim (get row "行番號"))
                               "candidate_no" (str/trim (get row "候補"))
                               "note" (str/trim (get row "備考"))}))))
                {}
                rows)
        [char-map ambiguous unchanged]
        (reduce (fn [[char-map ambiguous unchanged] [source candidates]]
                  (let [targets (set (map #(get % "target") candidates))]
                    (if (= 1 (count targets))
                      (let [target (first targets)]
                        (if (nil? target)
                          [char-map ambiguous (conj unchanged source)]
                          [(assoc char-map source target) ambiguous unchanged]))
                      [char-map
                       (assoc ambiguous source candidates)
                       unchanged])))
                [{} {} []]
                grouped)]
    {"source_url" source-url
     "generated_at" (-> (OffsetDateTime/now ZoneOffset/UTC)
                        (.truncatedTo ChronoUnit/MICROS)
                        (.format python-date-formatter))
     "row_count" (count rows)
     "input_csv" input-csv
     "char_map" char-map
     "ambiguous_characters" ambiguous
     "unchanged_characters" (vec (sort code-point-compare unchanged))}))


(defn main
  []
  (let [{:keys [csv help output source-url]} (parse-args)]
    (if help
      (do
        (println usage)
        0)
      (let [csv-text (if csv
                       (str/replace-first (slurp (str csv)) "\uFEFF" "")
                       (fetch-csv source-url))
            snapshot (build-snapshot (load-rows csv-text)
                                     source-url
                                     (some-> csv str))]
        (fs/create-dirs (fs/parent output))
        (spit (str output) (str (python-json snapshot 0) "\n"))
        (when (Files/isSameFile output default-output)
          (update-manifest output))
        (println (str "wrote " output))
        (println (format "char_map=%d ambiguous=%d unchanged=%d"
                         (count (get snapshot "char_map"))
                         (count (get snapshot "ambiguous_characters"))
                         (count (get snapshot "unchanged_characters"))))
        0))))


(System/exit (main))
