# Platform automation

COM/OLE is not part of the Valo language or core value model. Core activation,
dispatch, enumeration and VARIANT marshalling have been removed. CreateObject and
GetObject are no longer intrinsics.

A future Windows platform library may offer explicit automation APIs. No such
Valo library is implemented yet. Native Declare remains available for ordinary
platform C APIs.
