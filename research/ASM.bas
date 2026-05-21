Attribute VB_Name = "ASM"
' ============================================================================
' ASM - v1.0.1
' Copyright (c) 2026 UesleiDev
'
' Permission is hereby granted, free of charge, to any person obtaining a
' copy of this software and associated documentation files (the "Software"),
' to deal in the Software without restriction, including without limitation
' the rights to use, copy, modify, merge, publish, distribute, sublicense,
' and/or sell copies of the Software, and to permit persons to whom the
' Software is furnished to do so, subject to the following conditions:
'
' The above copyright notice and this permission notice shall be included in
' all copies or substantial portions of the Software.
'
' THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
' IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
' FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
' AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
' LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
' FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
' DEALINGS IN THE SOFTWARE.
' ============================================================================

Option Explicit
Option Private Module

Public Enum AsmOS
    osWindows = 1
    osMacOS = 2
End Enum

Public Enum AsmArchitecture
    archWinX86 = 1
    archWinX64 = 2
    archMacUnknown = 3
End Enum

Public Enum AsmError
    asmErrSuccess = 0
    asmErrFileNotFound = 1
    asmErrInvalidHeader = 2
    asmErrArchMismatch = 3
    asmErrMemAlloc = 4
    asmErrEmptyFile = 5
    asmErrMacNotSupported = 6
    asmErrNullPointer = 7
    asmErrInvalidSize = 8
    asmErrExecutionFailed = 9
    asmErrFileAccess = 10
End Enum

#If Mac Then
    #If VBA7 Then
        Private Declare PtrSafe Function mmap Lib "libc.dylib" (ByVal addr As LongPtr, ByVal length As LongPtr, ByVal prot As Long, ByVal flags As Long, ByVal fd As Long, ByVal offset As LongPtr) As LongPtr
        Private Declare PtrSafe Function mprotect Lib "libc.dylib" (ByVal addr As LongPtr, ByVal length As LongPtr, ByVal prot As Long) As Long
        Private Declare PtrSafe Function munmap Lib "libc.dylib" (ByVal addr As LongPtr, ByVal length As LongPtr) As Long
        Private Declare PtrSafe Sub memmove Lib "libc.dylib" (ByVal dest As LongPtr, ByVal src As LongPtr, ByVal size As LongPtr)
        Private Declare PtrSafe Sub memset Lib "libc.dylib" (ByVal dest As LongPtr, ByVal val As Long, ByVal size As LongPtr)
    #Else
        Private Declare Function mmap Lib "libc.dylib" (ByVal addr As Long, ByVal length As Long, ByVal prot As Long, ByVal flags As Long, ByVal fd As Long, ByVal offset As Long) As Long
        Private Declare Function mprotect Lib "libc.dylib" (ByVal addr As Long, ByVal length As Long, ByVal prot As Long) As Long
        Private Declare Function munmap Lib "libc.dylib" (ByVal addr As Long, ByVal length As Long) As Long
        Private Declare Sub memmove Lib "libc.dylib" (ByVal dest As Long, ByVal src As Long, ByVal size As Long)
        Private Declare Sub memset Lib "libc.dylib" (ByVal dest As Long, ByVal val As Long, ByVal size As Long)
    #End If
    Private Const PROT_READ As Long = 1
    Private Const PROT_WRITE As Long = 2
    Private Const PROT_EXEC As Long = 4
    Private Const MAP_PRIVATE As Long = 2
    Private Const MAP_ANON As Long = &H1000
#Else
    #If VBA7 Then
        Private Declare PtrSafe Function VirtualAlloc Lib "kernel32" (ByVal lpAddress As LongPtr, ByVal dwSize As LongPtr, ByVal flAllocationType As Long, ByVal flProtect As Long) As LongPtr
        Private Declare PtrSafe Function VirtualProtect Lib "kernel32" (ByVal lpAddress As LongPtr, ByVal dwSize As LongPtr, ByVal flNewProtect As Long, ByRef lpflOldProtect As Long) As Long
        Private Declare PtrSafe Function VirtualFree Lib "kernel32" (ByVal lpAddress As LongPtr, ByVal dwSize As LongPtr, ByVal dwFreeType As Long) As Long
        Public Declare PtrSafe Sub RtlMoveMemory Lib "kernel32" (ByVal Destination As LongPtr, ByVal Source As LongPtr, ByVal length As LongPtr)
        Public Declare PtrSafe Sub RtlZeroMemory Lib "kernel32" (ByVal Destination As LongPtr, ByVal length As LongPtr)
        Private Declare PtrSafe Function CallWindowProcW Lib "user32" (ByVal lpPrevWndFunc As LongPtr, ByVal RCX_P1 As LongPtr, ByVal RDX_P2 As LongPtr, ByVal R8_P3 As LongPtr, ByVal R9_P4 As LongPtr) As LongPtr
    #Else
        Private Declare Function VirtualAlloc Lib "kernel32" (ByVal lpAddress As Long, ByVal dwSize As Long, ByVal flAllocationType As Long, ByVal flProtect As Long) As Long
        Private Declare Function VirtualProtect Lib "kernel32" (ByVal lpAddress As Long, ByVal dwSize As Long, ByVal flNewProtect As Long, ByRef lpflOldProtect As Long) As Long
        Private Declare Function VirtualFree Lib "kernel32" (ByVal lpAddress As Long, ByVal dwSize As Long, ByVal dwFreeType As Long) As Long
        Private Declare Sub RtlMoveMemory Lib "kernel32" (ByVal Destination As Long, ByVal Source As Long, ByVal Length As Long)
        Private Declare Sub RtlZeroMemory Lib "kernel32" (ByVal Destination As Long, ByVal Length As Long)
        Private Declare Function CallWindowProcW Lib "user32" (ByVal lpPrevWndFunc As Long, ByVal RCX_P1 As Long, ByVal RDX_P2 As Long, ByVal R8_P3 As Long, ByVal R9_P4 As Long) As Long
    #End If
    Private Const MEM_COMMIT As Long = &H1000
    Private Const MEM_RESERVE As Long = &H2000
    Private Const PAGE_READWRITE As Long = &H4
    Private Const PAGE_EXECUTE_READ As Long = &H20
    Private Const MEM_RELEASE As Long = &H8000&
#End If

Private m_LastError As AsmError

Public Function GetLastError() As AsmError
    GetLastError = m_LastError
End Function

Public Function GetErrorString(ByVal errCode As AsmError) As String
    Select Case errCode
        Case asmErrSuccess: GetErrorString = "Operation completed successfully."
        Case asmErrFileNotFound: GetErrorString = "Target binary file could not be located or is a directory."
        Case asmErrInvalidHeader: GetErrorString = "Invalid magic header: File format not recognized."
        Case asmErrArchMismatch: GetErrorString = "Architecture mismatch: Binary payload is incompatible with the host environment."
        Case asmErrMemAlloc: GetErrorString = "Memory allocation failed: Insufficient resources or access denied."
        Case asmErrEmptyFile: GetErrorString = "The specified payload file is empty or contains no executable data."
        Case asmErrMacNotSupported: GetErrorString = "Operation not supported: Execution is restricted on the Darwin/macOS kernel."
        Case asmErrNullPointer: GetErrorString = "Null pointer exception: Invalid memory address referenced."
        Case asmErrInvalidSize: GetErrorString = "Invalid buffer size or uninitialized byte array."
        Case asmErrExecutionFailed: GetErrorString = "Execution failed: The host thread encountered an unhandled exception."
        Case asmErrFileAccess: GetErrorString = "Access denied: The target file is currently locked or access privileges are insufficient."
        Case Else: GetErrorString = "Unknown fatal error occurred within the assembly host."
    End Select
End Function

Public Function GetOS() As AsmOS
    #If Mac Then
        GetOS = osMacOS
    #Else
        GetOS = osWindows
    #End If
End Function

Public Function GetArch() As AsmArchitecture
    #If Mac Then
        GetArch = archMacUnknown
    #ElseIf Win64 Then
        GetArch = archWinX64
    #Else
        GetArch = archWinX86
    #End If
End Function

#If VBA7 Then

Public Function Alloc(ByVal size As LongPtr) As LongPtr
    m_LastError = asmErrSuccess
    If size <= 0 Then
        m_LastError = asmErrInvalidSize
        Exit Function
    End If
    #If Mac Then
        Alloc = mmap(0, size, PROT_READ Or PROT_WRITE, MAP_PRIVATE Or MAP_ANON, -1, 0)
        If Alloc = -1 Then Alloc = 0
    #Else
        Alloc = VirtualAlloc(0, size, MEM_COMMIT Or MEM_RESERVE, PAGE_READWRITE)
    #End If
    If Alloc = 0 Then m_LastError = asmErrMemAlloc
End Function

Public Sub MakeExecutable(ByVal Address As LongPtr, ByVal size As LongPtr)
    If Address = 0 Or size <= 0 Then Exit Sub
    #If Mac Then
        mprotect Address, size, PROT_READ Or PROT_EXEC
    #Else
        Dim OldP As Long
        VirtualProtect Address, size, PAGE_EXECUTE_READ, OldP
    #End If
End Sub

Public Sub Free(ByVal Address As LongPtr, ByVal size As LongPtr, Optional ByVal SecureWipe As Boolean = False)
    If Address = 0 Then Exit Sub
    If SecureWipe And size > 0 Then
        #If Mac Then
            memset Address, 0, size
        #Else
            RtlZeroMemory Address, size
        #End If
    End If
    #If Mac Then
        munmap Address, size
    #Else
        VirtualFree Address, 0, MEM_RELEASE
    #End If
End Sub

Public Sub InjectBytes(ByVal Address As LongPtr, ByRef Bytes() As Byte)
    Dim length As LongPtr
    If Address = 0 Then
        m_LastError = asmErrNullPointer
        Exit Sub
    End If
    If (Not Not Bytes) <> 0 Then
        length = UBound(Bytes) - LBound(Bytes) + 1
        If length <= 0 Then Exit Sub
        #If Mac Then
            memmove Address, VarPtr(Bytes(LBound(Bytes))), length
        #Else
            RtlMoveMemory Address, VarPtr(Bytes(LBound(Bytes))), length
        #End If
    Else
        m_LastError = asmErrInvalidSize
    End If
End Sub

Public Sub InjectPointer(ByVal Address As LongPtr, ByVal Value As LongPtr, ByVal BytesCount As LongPtr)
    If Address = 0 Then Exit Sub
    If BytesCount <= 0 Then Exit Sub
    #If Mac Then
        memmove Address, VarPtr(Value), BytesCount
    #Else
        RtlMoveMemory Address, VarPtr(Value), BytesCount
    #End If
End Sub

Public Function Run(ByVal Address As LongPtr, Optional ByVal RCX_P1 As LongPtr = 0, Optional ByVal RDX_P2 As LongPtr = 0, Optional ByVal R8_P3 As LongPtr = 0, Optional ByVal R9_P4 As LongPtr = 0) As LongPtr
    m_LastError = asmErrSuccess
    If Address = 0 Then
        m_LastError = asmErrNullPointer
        Exit Function
    End If
    #If Mac Then
        Run = 0
        m_LastError = asmErrMacNotSupported
    #Else
        Run = CallWindowProcW(Address, RCX_P1, RDX_P2, R8_P3, R9_P4)
    #End If
End Function

Public Function DumpMemory(ByVal Address As LongPtr, ByVal size As LongPtr) As String
    If Address = 0 Or size <= 0 Then Exit Function
    Dim Buffer() As Byte
    ReDim Buffer(0 To CLng(size) - 1)
    #If Mac Then
        memmove VarPtr(Buffer(0)), Address, size
    #Else
        RtlMoveMemory VarPtr(Buffer(0)), Address, size
    #End If
    Dim i As Long
    Dim result As String
    Dim HexStr As String
    Dim AscStr As String
    For i = 0 To UBound(Buffer)
        If i Mod 16 = 0 Then
            If i > 0 Then result = result & HexStr & "  | " & AscStr & vbCrLf
            HexStr = Right$("00000000" & Hex$(Address + i), 8) & ": "
            AscStr = ""
        End If
        HexStr = HexStr & Right$("00" & Hex$(Buffer(i)), 2) & " "
        If Buffer(i) >= 32 And Buffer(i) <= 126 Then
            AscStr = AscStr & Chr$(Buffer(i))
        Else
            AscStr = AscStr & "."
        End If
    Next i
    If Len(AscStr) > 0 Then
        HexStr = HexStr & Space$((16 - (Len(AscStr))) * 3)
        result = result & HexStr & "  | " & AscStr
    End If
    DumpMemory = result
End Function

Public Function CreateTrampoline(ByVal TargetAddress As LongPtr) As LongPtr
    Dim MemSize As LongPtr
    Dim MemAddr As LongPtr
    If GetArch() = archWinX64 Then
        MemSize = 12
        MemAddr = Alloc(MemSize)
        If MemAddr = 0 Then Exit Function
        Dim Code64(0 To 11) As Byte
        Code64(0) = &H48: Code64(1) = &HB8
        Code64(10) = &HFF: Code64(11) = &HE0
        InjectBytes MemAddr, Code64
        InjectPointer MemAddr + 2, TargetAddress, 8
    Else
        MemSize = 7
        MemAddr = Alloc(MemSize)
        If MemAddr = 0 Then Exit Function
        Dim Code32(0 To 6) As Byte
        Code32(0) = &HB8
        Code32(5) = &HFF: Code32(6) = &HE0
        InjectBytes MemAddr, Code32
        InjectPointer MemAddr + 1, TargetAddress, 4
    End If
    MakeExecutable MemAddr, MemSize
    CreateTrampoline = MemAddr
End Function

Public Function LoadAndRunBin(ByVal FilePath As String, Optional ByVal RCX_P1 As LongPtr = 0, Optional ByVal RDX_P2 As LongPtr = 0, Optional ByVal R8_P3 As LongPtr = 0, Optional ByVal R9_P4 As LongPtr = 0) As LongPtr
    m_LastError = asmErrSuccess
    #If Mac Then
        m_LastError = asmErrMacNotSupported
        Exit Function
    #End If
    Dim arch As AsmArchitecture
    arch = GetArch()
    If arch = archMacUnknown Then
        m_LastError = asmErrMacNotSupported
        Exit Function
    End If
    If Len(Dir(FilePath)) = 0 Then
        m_LastError = asmErrFileNotFound
        Exit Function
    End If
    If (GetAttr(FilePath) And vbDirectory) = vbDirectory Then
        m_LastError = asmErrFileNotFound
        Exit Function
    End If
    Dim fSize As Long
    fSize = VBA.fileLen(FilePath)
    If fSize = 0 Then
        m_LastError = asmErrEmptyFile
        Exit Function
    End If
    Dim fileNum As Integer
    fileNum = FreeFile
    Open FilePath For Binary Access Read Shared As fileNum
    Dim magic(0 To 1) As Byte
    Get fileNum, 1, magic
    Dim useHeader As Boolean
    useHeader = (magic(0) = 65 And magic(1) = 83)
    Dim fileArch As Byte
    Dim codeStart As Long
    If useHeader Then
        Get fileNum, 3, fileArch
        codeStart = 4
        If fileArch < 1 Or fileArch > 2 Then
            Close fileNum
            m_LastError = asmErrInvalidHeader
            Exit Function
        End If
        If (fileArch = 1 And arch <> archWinX86) Or (fileArch = 2 And arch <> archWinX64) Then
            Close fileNum
            m_LastError = asmErrArchMismatch
            Exit Function
        End If
    Else
        codeStart = 1
    End If
    fSize = LOF(fileNum) - (codeStart - 1)
    If fSize <= 0 Then
        Close fileNum
        m_LastError = asmErrEmptyFile
        Exit Function
    End If
    Dim codeBytes() As Byte
    ReDim codeBytes(0 To fSize - 1)
    Get fileNum, codeStart, codeBytes
    Close fileNum
    Dim pMem As LongPtr
    Dim sz As LongPtr
    sz = fSize
    pMem = Alloc(sz)
    If pMem = 0 Then
        m_LastError = asmErrMemAlloc
        Exit Function
    End If
    InjectBytes pMem, codeBytes
    MakeExecutable pMem, sz
    LoadAndRunBin = Run(pMem, RCX_P1, RDX_P2, R8_P3, R9_P4)
    Free pMem, sz
End Function

#Else

Public Function Alloc(ByVal size As Long) As Long
    m_LastError = asmErrSuccess
    If size <= 0 Then
        m_LastError = asmErrInvalidSize
        Exit Function
    End If
    #If Mac Then
        Alloc = mmap(0, size, PROT_READ Or PROT_WRITE, MAP_PRIVATE Or MAP_ANON, -1, 0)
        If Alloc = -1 Then Alloc = 0
    #Else
        Alloc = VirtualAlloc(0, size, MEM_COMMIT Or MEM_RESERVE, PAGE_READWRITE)
    #End If
    If Alloc = 0 Then m_LastError = asmErrMemAlloc
End Function

Public Sub MakeExecutable(ByVal Address As Long, ByVal size As Long)
    If Address = 0 Or size <= 0 Then Exit Sub
    #If Mac Then
        mprotect Address, size, PROT_READ Or PROT_EXEC
    #Else
        Dim OldP As Long
        VirtualProtect Address, size, PAGE_EXECUTE_READ, OldP
    #End If
End Sub

Public Sub Free(ByVal Address As Long, ByVal size As Long, Optional ByVal SecureWipe As Boolean = False)
    If Address = 0 Then Exit Sub
    If SecureWipe And size > 0 Then
        #If Mac Then
            memset Address, 0, size
        #Else
            RtlZeroMemory Address, size
        #End If
    End If
    #If Mac Then
        munmap Address, size
    #Else
        VirtualFree Address, 0, MEM_RELEASE
    #End If
End Sub

Public Sub InjectBytes(ByVal Address As Long, ByRef Bytes() As Byte)
    Dim length As Long
    If Address = 0 Then
        m_LastError = asmErrNullPointer
        Exit Sub
    End If
    If (Not Not Bytes) <> 0 Then
        length = UBound(Bytes) - LBound(Bytes) + 1
        If length <= 0 Then Exit Sub
        #If Mac Then
            memmove Address, VarPtr(Bytes(LBound(Bytes))), length
        #Else
            RtlMoveMemory Address, VarPtr(Bytes(LBound(Bytes))), length
        #End If
    Else
        m_LastError = asmErrInvalidSize
    End If
End Sub

Public Sub InjectPointer(ByVal Address As Long, ByVal Value As Long, ByVal BytesCount As Long)
    If Address = 0 Then Exit Sub
    If BytesCount <= 0 Then Exit Sub
    #If Mac Then
        memmove Address, VarPtr(Value), BytesCount
    #Else
        RtlMoveMemory Address, VarPtr(Value), BytesCount
    #End If
End Sub

Public Function Run(ByVal Address As Long, Optional ByVal RCX_P1 As Long = 0, Optional ByVal RDX_P2 As Long = 0, Optional ByVal R8_P3 As Long = 0, Optional ByVal R9_P4 As Long = 0) As Long
    m_LastError = asmErrSuccess
    If Address = 0 Then
        m_LastError = asmErrNullPointer
        Exit Function
    End If
    #If Mac Then
        Run = 0
        m_LastError = asmErrMacNotSupported
    #Else
        Run = CallWindowProcW(Address, RCX_P1, RDX_P2, R8_P3, R9_P4)
    #End If
End Function

Public Function DumpMemory(ByVal Address As Long, ByVal size As Long) As String
    If Address = 0 Or size <= 0 Then Exit Function
    Dim Buffer() As Byte
    ReDim Buffer(0 To size - 1)
    #If Mac Then
        memmove VarPtr(Buffer(0)), Address, size
    #Else
        RtlMoveMemory VarPtr(Buffer(0)), Address, size
    #End If
    Dim i As Long
    Dim result As String
    Dim HexStr As String
    Dim AscStr As String
    For i = 0 To UBound(Buffer)
        If i Mod 16 = 0 Then
            If i > 0 Then result = result & HexStr & "  | " & AscStr & vbCrLf
            HexStr = Right$("00000000" & Hex$(Address + i), 8) & ": "
            AscStr = ""
        End If
        HexStr = HexStr & Right$("00" & Hex$(Buffer(i)), 2) & " "
        If Buffer(i) >= 32 And Buffer(i) <= 126 Then
            AscStr = AscStr & Chr$(Buffer(i))
        Else
            AscStr = AscStr & "."
        End If
    Next i
    If Len(AscStr) > 0 Then
        HexStr = HexStr & Space$((16 - (Len(AscStr))) * 3)
        result = result & HexStr & "  | " & AscStr
    End If
    DumpMemory = result
End Function

Public Function CreateTrampoline(ByVal TargetAddress As Long) As Long
    Dim MemSize As Long
    Dim MemAddr As Long
    If GetArch() = archWinX64 Then
        MemSize = 12
        MemAddr = Alloc(MemSize)
        If MemAddr = 0 Then Exit Function
        Dim Code64(0 To 11) As Byte
        Code64(0) = &H48: Code64(1) = &HB8
        Code64(10) = &HFF: Code64(11) = &HE0
        InjectBytes MemAddr, Code64
        InjectPointer MemAddr + 2, TargetAddress, 8
    Else
        MemSize = 7
        MemAddr = Alloc(MemSize)
        If MemAddr = 0 Then Exit Function
        Dim Code32(0 To 6) As Byte
        Code32(0) = &HB8
        Code32(5) = &HFF: Code32(6) = &HE0
        InjectBytes MemAddr, Code32
        InjectPointer MemAddr + 1, TargetAddress, 4
    End If
    MakeExecutable MemAddr, MemSize
    CreateTrampoline = MemAddr
End Function

Public Function LoadAndRunBin(ByVal FilePath As String, Optional ByVal RCX_P1 As Long = 0, Optional ByVal RDX_P2 As Long = 0, Optional ByVal R8_P3 As Long = 0, Optional ByVal R9_P4 As Long = 0) As Long
    m_LastError = asmErrSuccess
    #If Mac Then
        m_LastError = asmErrMacNotSupported
        Exit Function
    #End If
    Dim arch As AsmArchitecture
    arch = GetArch()
    If arch = archMacUnknown Then
        m_LastError = asmErrMacNotSupported
        Exit Function
    End If
    If Len(Dir(FilePath)) = 0 Then
        m_LastError = asmErrFileNotFound
        Exit Function
    End If
    If (GetAttr(FilePath) And vbDirectory) = vbDirectory Then
        m_LastError = asmErrFileNotFound
        Exit Function
    End If
    Dim fSize As Long
    fSize = VBA.fileLen(FilePath)
    If fSize = 0 Then
        m_LastError = asmErrEmptyFile
        Exit Function
    End If
    Dim fileNum As Integer
    fileNum = FreeFile
    Open FilePath For Binary Access Read Shared As fileNum
    Dim magic(0 To 1) As Byte
    Get fileNum, 1, magic
    Dim useHeader As Boolean
    useHeader = (magic(0) = 65 And magic(1) = 83)
    Dim fileArch As Byte
    Dim codeStart As Long
    If useHeader Then
        Get fileNum, 3, fileArch
        codeStart = 4
        If fileArch < 1 Or fileArch > 2 Then
            Close fileNum
            m_LastError = asmErrInvalidHeader
            Exit Function
        End If
        If (fileArch = 1 And arch <> archWinX86) Or (fileArch = 2 And arch <> archWinX64) Then
            Close fileNum
            m_LastError = asmErrArchMismatch
            Exit Function
        End If
    Else
        codeStart = 1
    End If
    fSize = LOF(fileNum) - (codeStart - 1)
    If fSize <= 0 Then
        Close fileNum
        m_LastError = asmErrEmptyFile
        Exit Function
    End If
    Dim codeBytes() As Byte
    ReDim codeBytes(0 To fSize - 1)
    Get fileNum, codeStart, codeBytes
    Close fileNum
    Dim pMem As Long
    Dim sz As Long
    sz = fSize
    pMem = Alloc(sz)
    If pMem = 0 Then
        m_LastError = asmErrMemAlloc
        Exit Function
    End If
    InjectBytes pMem, codeBytes
    MakeExecutable pMem, sz
    LoadAndRunBin = Run(pMem, RCX_P1, RDX_P2, R8_P3, R9_P4)
    Free pMem, sz
End Function
#End If

Public Function GenerateVBACodeFromFile(ByVal FilePath As String, Optional ByVal ArrayName As String = "opcodes") As String
    Dim fileNum As Integer
    Dim fSize As Long
    Dim Bytes() As Byte
    Dim i As Long
    Dim result As String
    Dim chunk As String
    Dim hexVal As String
    
    If Len(Dir(FilePath)) = 0 Then
        GenerateVBACodeFromFile = "Error: Target file not found."
        Exit Function
    End If
    
    If (GetAttr(FilePath) And vbDirectory) = vbDirectory Then
        GenerateVBACodeFromFile = "Error: Specified path is a directory."
        Exit Function
    End If
    
    fSize = VBA.fileLen(FilePath)
    If fSize = 0 Then
        GenerateVBACodeFromFile = "Error: File is empty."
        Exit Function
    End If
    
    fileNum = FreeFile
    Open FilePath For Binary Access Read Shared As fileNum
    ReDim Bytes(0 To fSize - 1)
    Get fileNum, 1, Bytes
    Close fileNum
    
    result = "ReDim " & ArrayName & "(0 To " & CStr(fSize - 1) & ")" & vbCrLf
    For i = 0 To fSize - 1
        hexVal = Right$("00" & Hex$(Bytes(i)), 2)
        
        chunk = chunk & ArrayName & "(" & CStr(i) & ") = &H" & hexVal
        
        If i < fSize - 1 Then
            If (i + 1) Mod 5 = 0 Then
                result = result & chunk & vbCrLf
                chunk = ""
            Else
                chunk = chunk & ": "
            End If
        Else
            result = result & chunk
        End If
    Next i
    
    GenerateVBACodeFromFile = result
End Function
