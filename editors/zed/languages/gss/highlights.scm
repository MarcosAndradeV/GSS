(entry
  key: (identifier) @property)

(access
  (identifier) @variable)

(relative_access
  (identifier) @variable.member)

(boolean) @boolean

(number) @number
(float) @number.float
(percentage) @number

(string) @string
(escape_sequence) @string.escape

(comment) @comment

"=" @operator
"," @punctuation.delimiter
"." @punctuation.delimiter

"{" @punctuation.bracket
"}" @punctuation.bracket
"[" @punctuation.bracket
"]" @punctuation.bracket
