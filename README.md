# TabletopBot
This is a discord bot which provides a way to define and perform calculations (primarily those necessary for a game of D&amp;D 3.5). It can also port over character sheets from the website myth-weavers.com

The query language is actually decently complex. Here are a few highlights
- Five basic math operators (+,-,*,/,^)
- Dice rolling
- Variable declaration, including both setting a variable to a particlar value and aliasing an expression as it
- Strings are a core part of the language in order to pretty up any results if desired
- Ternary statement for more complex query logic
- Indirect variable addressing. For example you can store a string "y" in a variable $x and $($x) will evaluate to $y

There's also a very simple key-value database implementation that should eventually be replaced with Redis or something like it.
