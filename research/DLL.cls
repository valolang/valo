VERSION 1.0 CLASS
BEGIN
  MultiUse = -1  'True
END
Attribute VB_Name = "DLL"
Attribute VB_GlobalNameSpace = False
Attribute VB_Creatable = False
Attribute VB_PredeclaredId = False
Attribute VB_Exposed = False
Attribute VB_Description = "BetterDLL is a Class Module for help using DLLs and APIs better."
' ===============================================================================
'  BetterDLL – Dynamic DLL Loader and Function Dispatcher for VBA
' ===============================================================================
'
'  Author    : UesleiDev
'  Version   : 1.3
'  Last Upd. : ?? ?? 2024
'  Language  : VBA (64-bit compatible via PtrSafe)
'
'  OVERVIEW
'  -------------------------------------------------------------------------------
'  BetterDLL is a lightweight DLL wrapper for VBA, providing runtime loading of
'  external libraries and dynamic invocation of exported functions. It abstracts
'  the use of `LoadLibrary`, `FreeLibrary`, `GetProcAddress`, and `DispCallFunc`
'  into a higher-level interface, enabling developers to bind and call DLL
'  functions without compile-time declarations.
'
'
'  KEY FEATURES
'  -------------------------------------------------------------------------------
'  • Library Management:
'      - `Create`: Loads a specified DLL from Windows System32 or a custom path.
'      - `FreeDLL`: Unloads the library and releases resources.
'
'  • Function Binding:
'      - `Add`: Registers a DLL function by name and expected return type.
'      - Functions are stored internally in a Dictionary (name ? address, type).
'
'  • Invocation:
'      - `Run`: Executes a previously registered function with optional parameters.
'      - Supports automatic mapping of strings and numeric arguments.
'      - Handles return values via `DispCallFunc` into a Variant.
'
'  • Error Handling:
'      - Graceful handling when libraries, functions, or directories are not found.
'      - Debug messages for missing initialization or duplicate entries.
'      - Explicit error raising through `RaiseError`.
'
'
'  LIMITATIONS
'  -------------------------------------------------------------------------------
'  • Function parameters must be handled with care (conversion/pointers).
'  • No type marshaling beyond strings and primitive numeric values.
'  • Complex structures, callbacks, or object pointers are not supported.
'  • Misdeclared return/parameter types may result in runtime errors or crashes.
'
'
'  PERFORMANCE NOTES
'  -------------------------------------------------------------------------------
'  • Uses Windows API functions (`LoadLibrary`, `GetProcAddress`, `FreeLibrary`).
'  • Relies on `DispCallFunc` for flexible runtime invocation.
'  • Suitable for lightweight, dynamic DLL calls; not a substitute for a full
'    COM or type-library interface.
'
' ===============================================================================

Option Explicit

Private Declare PtrSafe Function DispCallFunc Lib "oleaut32.dll" (ByVal pvInstance As Long, ByVal offsetinVft As LongPtr, ByVal CallConv As Long, ByVal retTYP As Integer, ByVal paCNT As Long, ByRef paTypes As Integer, ByRef paValues As LongPtr, ByRef retVAR As Variant) As Long
Private Declare PtrSafe Function LoadLibrary Lib "kernel32.dll" Alias "LoadLibraryA" (ByVal lpLibFileName As String) As LongPtr
Private Declare PtrSafe Function GetModuleHandle Lib "kernel32.dll" Alias "GetModuleHandleA" (ByVal lpModuleName As String) As LongPtr
Private Declare PtrSafe Function FreeLibrary Lib "kernel32.dll" (ByVal hLibModule As LongPtr) As LongPtr
Private Declare PtrSafe Function GetProcAddress Lib "kernel32.dll" (ByVal hModule As LongPtr, ByVal lpProcName As String) As LongPtr

Private dll_lib As LongPtr

Private loaded As Boolean
Private functions As Dictionary

Public Sub FreeDLL()
    If dll_lib <> 0 Then
        FreeLibrary (dll_lib)
        loaded = False
    End If
End Sub

Public Sub Create(lib As String, Optional directory As String = vbNullString)
    If loaded Then
        Debug.Print "BetterDLL::Create - dll is already intialized, try use ""FreeDLL"" for unload."
        Exit Sub
    End If

    Set functions = New Dictionary

    If Len(directory) > 0 Then
        If Len(Dir(directory, vbDirectory)) = 0 Then
            RaiseError ("Directory """ & directory & """ not founded, verify if the path is correct.")
        End If
        ChDir (directory)
    Else
        ChDir ("C:\Windows\System32")
    End If
    
    dll_lib = GetModuleHandle(lib)
    If dll_lib = 0& Then
        dll_lib = LoadLibrary(lib)
        If dll_lib = 0& Then
            RaiseError ("DLL """ & lib & """ can't be loaded, try check the name or directory.")
            Exit Sub
        End If
    End If
    loaded = True
End Sub

Public Sub Add(name As String, return_type As VbVarType)
    If Not loaded Then
        Debug.Print "BetterDLL::Add(""" & name & """) - not initialized properly, check the previous alert and fix it."
        Exit Sub
    End If

    Dim func As LongPtr, func_items As New Dictionary
    
    If functions.Exists(name) Then
        Debug.Print "Function """ & name & """ " & "already exists."
        Exit Sub
    End If
    
    func = GetProcAddress(dll_lib, name)
    If func = 0& Then
        Debug.Print "Function """ & name & """ " & "not found on dll."
        Exit Sub
    End If
    
    func_items.Add "value", CLngPtr(func)
    func_items.Add "return_type", return_type
    functions.Add name, func_items
End Sub

Public Function Run(ByVal name As String, Optional params As Variant) As Variant
Attribute Run.VB_UserMemId = 0
    If Not loaded Then
        Debug.Print "BetterDLL::Run(""" & name & """) - not initialized properly, check the previous alert and fix it."
        Exit Function
    End If

    If Not functions.Exists(name) Then
        Debug.Print "Function """ & name & """ not found in the created functions."
        Exit Function
    End If
    
    Dim paramTypes() As Integer, paramDataPointers() As LongPtr
    Dim funcReturn As Long, funcResult As Variant
    Dim paramsCount As Long
    Dim stringPointers() As Variant, numberPointers() As Variant
    Dim i As Integer
    
    funcResult = Empty
    
    If IsMissing(params) Then
        paramsCount = 0
    ElseIf IsArray(params) Then
        paramsCount = UBound(params) + 1
    End If

    If paramsCount > 0 Then
        ReDim paramTypes(0 To paramsCount - 1)
        ReDim paramDataPointers(0 To paramsCount - 1)
        ReDim stringPointers(0 To paramsCount - 1)
        ReDim numberPointers(0 To paramsCount - 1)
        
        For i = 0 To paramsCount - 1
            paramTypes(i) = VarType(params(i))
        
            Select Case paramTypes(i)
                Case vbString
                    stringPointers(i) = StrConv(params(i), vbFromUnicode)
                    paramDataPointers(i) = VarPtr(stringPointers(i))
                Case Else
                    numberPointers(i) = CLngPtr(params(i))
                    paramDataPointers(i) = VarPtr(numberPointers(i))
            End Select
        Next i
    End If

    If paramsCount > 0 Then
        funcReturn = DispCallFunc(0&, functions(name)("value"), 4, functions(name)("return_type"), paramsCount, paramTypes(0), paramDataPointers(0), funcResult)
    Else
        funcReturn = DispCallFunc(0&, functions(name)("value"), 4, functions(name)("return_type"), 0, 0, 0, funcResult)
    End If
    
    If funcReturn = 0& Then
        Run = funcResult
    End If
End Function

Private Sub RaiseError(text As String)
    MsgBox text, vbCritical, "BetterDLL::RaiseError"
End Sub
