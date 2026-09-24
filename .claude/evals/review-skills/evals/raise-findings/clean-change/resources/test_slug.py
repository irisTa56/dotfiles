from slug import slugify


def test_lowercases_and_joins_words():
    assert slugify("Hello World") == "hello-world"


def test_drops_accents():
    assert slugify("Crème Brûlée") == "creme-brulee"


def test_collapses_and_trims_separators():
    assert slugify("  --a  b--  ") == "a-b"


def test_empty_input():
    assert slugify("") == ""
