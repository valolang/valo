Option Explicit

Private Declare PtrSafe Function VirtualAlloc Lib "kernel32" (ByVal lpAddress As LongPtr, ByVal dwSize As LongPtr, ByVal flAllocationType As Long, ByVal flProtect As Long) As LongPtr
Private Declare PtrSafe Sub RtlMoveMemory Lib "kernel32" (ByVal Destination As LongPtr, ByVal Source As LongPtr, ByVal Length As LongPtr)
Private Declare PtrSafe Function SetTimer Lib "user32" (ByVal hWnd As LongPtr, ByVal nIDEvent As LongPtr, ByVal uElapse As Long, ByVal lpTimerFunc As LongPtr) As LongPtr
Private Declare PtrSafe Function KillTimer Lib "user32" (ByVal hWnd As LongPtr, ByVal nIDEvent As LongPtr) As Long
Private Declare PtrSafe Sub Sleep Lib "kernel32" (ByVal dwMilliseconds As Long)
Private Declare PtrSafe Function GetModuleHandleA Lib "kernel32" (ByVal lpModuleName As String) As LongPtr
Private Declare PtrSafe Function GetProcAddress Lib "kernel32" (ByVal hModule As LongPtr, ByVal lpProcName As String) As LongPtr
Private Declare PtrSafe Function GetProcAddressOrdinal Lib "kernel32" Alias "GetProcAddress" (ByVal hModule As LongPtr, ByVal lpProcName As Long) As LongPtr

Private Const MEM_COMMIT As Long = &H1000
Private Const MEM_RESERVE As Long = &H2000
Private Const PAGE_EXECUTE_READWRITE As Long = &H40

Private Function DummyEbMode() As Long
    DummyEbMode = 1
End Function

Private Function GetAddressOf(ByVal ptr As LongPtr) As LongPtr
    GetAddressOf = ptr
End Function

Private Sub ProbeTimerCallback(ByVal hWnd As LongPtr, ByVal uMsg As Long, ByVal idEvent As LongPtr, ByVal dwTime As Long)
End Sub

Public Function BuildThunkOnly() As Boolean
    Const THUNK_SIZE As Long = 1024

    Dim thunkPtr As LongPtr
    Dim hVbe As LongPtr
    Dim pEbMode As LongPtr
    Dim pCallback As LongPtr
    Dim pKill As LongPtr
    Dim opcodes() As Byte
    Dim hexStr As String
    Dim i As Long

    thunkPtr = VirtualAlloc(0, THUNK_SIZE, MEM_COMMIT Or MEM_RESERVE, PAGE_EXECUTE_READWRITE)
    Console.WriteLine("alloc=" & (thunkPtr <> 0))

    hVbe = GetModuleHandleA("vbe7.dll")
    If hVbe = 0 Then
        hVbe = GetModuleHandleA("vba6.dll")
    End If
    Console.WriteLine("hVbe=" & (hVbe <> 0))

    If hVbe <> 0 Then
        pEbMode = GetProcAddress(hVbe, "EbMode")
        If pEbMode = 0 Then
            pEbMode = GetProcAddressOrdinal(hVbe, 1&)
        End If
    End If
    If pEbMode = 0 Then
        pEbMode = GetAddressOf(AddressOf DummyEbMode)
    End If
    Console.WriteLine("pEbMode=" & (pEbMode <> 0))

    pCallback = GetAddressOf(AddressOf ProbeTimerCallback)
    Console.WriteLine("pCallback=" & (pCallback <> 0))

    pKill = GetProcAddress(GetModuleHandleA("user32.dll"), "KillTimer")
    Console.WriteLine("pKill=" & (pKill <> 0))

    hexStr = "4883EC2848894C243048895424384C894424404C894C244848B80000000000000000" & _
             "4885C07429FFD083F801742283F802741D488B4C2430488B54244048B800000000" & _
             "000000004885C07429FFD0EB25488B4C2430488B5424384C8B4424404C8B4C2448" & _
             "48B800000000000000004885C07402FFD04883C428C3"

    ReDim opcodes(0 To (Len(hexStr) \ 2) - 1)
    For i = 0 To UBound(opcodes)
        opcodes(i) = CByte("&H" & Mid$(hexStr, (i * 2) + 1, 2))
    Next i
    Console.WriteLine("opcodes=" & (UBound(opcodes) + 1))

    RtlMoveMemory VarPtr(opcodes(26)), VarPtr(pEbMode), 8
    RtlMoveMemory VarPtr(opcodes(63)), VarPtr(pKill), 8
    RtlMoveMemory VarPtr(opcodes(102)), VarPtr(pCallback), 8
    Console.WriteLine("patch=ok")

    RtlMoveMemory ByVal thunkPtr, VarPtr(opcodes(0)), UBound(opcodes) + 1
    Console.WriteLine("copy=ok")

    BuildThunkOnly = True
End Function

Public Function BuildAndArmTimer() As Boolean
    Const THUNK_SIZE As Long = 1024

    Dim thunkPtr As LongPtr
    Dim timerId As LongPtr
    Dim hVbe As LongPtr
    Dim pEbMode As LongPtr
    Dim pCallback As LongPtr
    Dim pKill As LongPtr
    Dim opcodes() As Byte
    Dim hexStr As String
    Dim i As Long

    thunkPtr = VirtualAlloc(0, THUNK_SIZE, MEM_COMMIT Or MEM_RESERVE, PAGE_EXECUTE_READWRITE)
    If thunkPtr = 0 Then Exit Function

    hVbe = GetModuleHandleA("vbe7.dll")
    If hVbe = 0 Then
        hVbe = GetModuleHandleA("vba6.dll")
    End If

    If hVbe <> 0 Then
        pEbMode = GetProcAddress(hVbe, "EbMode")
        If pEbMode = 0 Then
            pEbMode = GetProcAddressOrdinal(hVbe, 1&)
        End If
    End If
    If pEbMode = 0 Then
        pEbMode = GetAddressOf(AddressOf DummyEbMode)
    End If

    pCallback = GetAddressOf(AddressOf ProbeTimerCallback)
    pKill = GetProcAddress(GetModuleHandleA("user32.dll"), "KillTimer")

    hexStr = "4883EC2848894C243048895424384C894424404C894C244848B80000000000000000" & _
             "4885C07429FFD083F801742283F802741D488B4C2430488B54244048B800000000" & _
             "000000004885C07429FFD0EB25488B4C2430488B5424384C8B4424404C8B4C2448" & _
             "48B800000000000000004885C07402FFD04883C428C3"

    ReDim opcodes(0 To (Len(hexStr) \ 2) - 1)
    For i = 0 To UBound(opcodes)
        opcodes(i) = CByte("&H" & Mid$(hexStr, (i * 2) + 1, 2))
    Next i

    RtlMoveMemory VarPtr(opcodes(26)), VarPtr(pEbMode), 8
    RtlMoveMemory VarPtr(opcodes(63)), VarPtr(pKill), 8
    RtlMoveMemory VarPtr(opcodes(102)), VarPtr(pCallback), 8
    RtlMoveMemory ByVal thunkPtr, VarPtr(opcodes(0)), UBound(opcodes) + 1
    Console.WriteLine("before-settimer")

    timerId = SetTimer(0, 0, 15, thunkPtr)
    Console.WriteLine("timer=" & (timerId <> 0))
    Sleep 50
    If timerId <> 0 Then
        KillTimer 0, timerId
    End If
    Console.WriteLine("after-kill")
    BuildAndArmTimer = True
End Function
