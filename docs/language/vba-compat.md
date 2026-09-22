# Legacy direction removed

Valo no longer has a VBA/VB6 source-compatibility commitment or project mode.
Exported .bas/.cls modules, compatibility manifest settings, core COM/OLE,
CreateObject/GetObject/DoEvents, PtrSafe, LongPtr and Set assignment are removed.

Preserve modern VB.NET syntax when porting Valo itself. Constructs shared with
VB.NET are not legacy-only by definition. The [audit](../architecture/native-migration.md)
records the remaining parser/runtime debt and the ordered migration plan.

This page is a migration notice, not a compatibility reference.
