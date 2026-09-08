#!/usr/bin/env bb
(ns verify-dic-manifest
  "dic/MANIFEST.jsonに記錄されたSHA-256を檢證する。"
  (:require
    [babashka.fs :as fs]
    [cheshire.core :as json])
  (:import
    (java.math
      BigInteger)
    (java.security
      MessageDigest)))


(def data-dir
  (-> *file*
      fs/path
      fs/parent
      fs/parent
      (fs/path "dic")))


(def manifest-path (fs/path data-dir "MANIFEST.json"))


(defn sha256
  [path]
  (let [digest (MessageDigest/getInstance "SHA-256")]
    (format "%064x" (BigInteger. 1 (.digest digest (fs/read-all-bytes path))))))


(defn main
  []
  (let [manifest (json/parse-string (slurp (str manifest-path)))
        invalid? (atom false)]
    (doseq [[name expected] (get manifest "files")]
      (let [actual (sha256 (fs/path data-dir name))]
        (when (not= actual expected)
          (binding [*out* *err*]
            (println (format "%s: expected %s, got %s" name expected actual)))
          (reset! invalid? true))))
    (if @invalid? 1 0)))


(System/exit (main))
