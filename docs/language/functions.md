# Functions and Argument Passing

Valo keeps VBA-compatible parameter defaults: omitted `ByVal`/`ByRef` is parsed as `ByRef`, but expression arguments, literals, coercions, and incompatible variable types are passed through temporary copy values where VBA would commonly do so.

Optional parameters preserve omitted state for `IsMissing`. If an omitted optional value is used where a concrete value is required, diagnostics explain that the optional argument was omitted instead of reporting a generic variable error.
