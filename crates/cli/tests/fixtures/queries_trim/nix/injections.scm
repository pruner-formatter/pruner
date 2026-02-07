; #-style Comments
((comment) @injection.language
  . ; this is to make sure only adjacent comments are accounted for the injections
  (string_expression
    (string_fragment) @injection.content)
  (#gsub! @injection.language "#%s*([%w%p]+)%s*" "%1")
  (#set! injection.combined))

((comment) @injection.language
  . ; this is to make sure only adjacent comments are accounted for the injections
  (indented_string_expression
    (string_fragment) @injection.content)
  (#gsub! @injection.language "#%s*([%w%p]+)%s*" "%1")
  (#trim! @injection.content 1 0 1 0)
  (#set! injection.combined))
