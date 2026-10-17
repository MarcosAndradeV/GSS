module.exports = grammar({
  name: 'gss',

  extras: $ => [
    /\s+/,
    $.comment,
  ],

  rules: {
    source_file: $ => repeat($.entry),

    entry: $ => seq(
      field('key', $.identifier),
      '=',
      field('value', $._value),
      optional(',')
    ),

    _value: $ => choice(
      $.object,
      $.list,
      $.boolean,
      $.percentage,
      $.float,
      $.number,
      $.string,
      $.relative_access,
      $.access,
      $.identifier,
    ),

    object: $ => seq(
      '{',
      repeat($.entry),
      '}'
    ),

    list: $ => seq(
      '[',
      optional(seq(
        $._value,
        repeat(seq(',', $._value)),
        optional(',')
      )),
      ']'
    ),

    access: $ => prec(2, seq(
      $.identifier,
      repeat1(seq('.', $.identifier))
    )),

    relative_access: $ => prec(2, seq(
      '.',
      $.identifier,
      repeat(seq('.', $.identifier))
    )),

    boolean: _ => choice('true', 'false'),

    percentage: _ => token(seq(
      optional(/[+-]/),
      choice(
        /[0-9]+\.[0-9]+/,
        /[0-9]+/,
        /0x[0-9a-fA-F]+/
      ),
      '%'
    )),

    number: _ => token(seq(
      optional(/[+-]/),
      choice(
        /0x[0-9a-fA-F]+/,
        /[0-9]+/
      )
    )),

    float: _ => token(seq(optional(/[+-]/), /[0-9]+\.[0-9]+/)),

    string: $ => seq(
      '"',
      repeat(choice(
        $.escape_sequence,
        /[^"\\\n]+/
      )),
      '"'
    ),

    escape_sequence: _ => token(seq(
      '\\',
      choice(
        /[\\"/bfnrt]/,
        /u[0-9a-fA-F]{4}/
      )
    )),

    identifier: _ => /[a-zA-Z_][a-zA-Z0-9_]*/,

    comment: _ => token(choice(
      seq('//', /.*/),
      seq('/*', /[^*]*\*+([^/*][^*]*\*+)*/, '/')
    )),
  }
});
