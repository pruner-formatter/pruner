(defn somefunc
   "2-byte UTF-8: Cyrillic (кириллица) and accents (café, señor, über)"
   [ ]
   (print "hello"))

(defn another-func
   "3-byte UTF-8: Chinese (中文), Japanese (日本語), Korean (한글)"
     []
       nil)

(defn emoji-func
   "4-byte UTF-8: Emojis 🚀🎉👨‍💻 and symbols 𝕳𝖊𝖑𝖑𝖔"
 []
   :ok)

(defn mixed-func
   "Mixed: Привет мир! 你好世界! 🌍 γειά σου κόσμε"
     []
 true)
