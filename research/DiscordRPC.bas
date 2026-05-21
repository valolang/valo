Attribute VB_Name = "DiscordRPC"
' -----------------------------------------------------------------------------
' DiscordRPC v2
' -----------------------------------------------------------------------------
' Team: Bedrock
' Version: 1.1
' Last Update: 01/01/2026
' -----------------------------------------------------------------------------

Option Explicit

Private Declare PtrSafe Function CreateFile Lib "kernel32" Alias "CreateFileA" ( _
    ByVal lpFileName As String, _
    ByVal dwDesiredAccess As Long, _
    ByVal dwShareMode As Long, _
    ByVal lpSecurityAttributes As LongPtr, _
    ByVal dwCreationDisposition As Long, _
    ByVal dwFlagsAndAttributes As Long, _
    ByVal hTemplateFile As LongPtr) As LongPtr

Private Declare PtrSafe Function WriteFile Lib "kernel32" ( _
    ByVal hFile As LongPtr, _
    lpBuffer As Any, _
    ByVal nNumberOfBytesToWrite As Long, _
    lpNumberOfBytesWritten As Long, _
    ByVal lpOverlapped As LongPtr) As Long

Private Declare PtrSafe Function ReadFile Lib "kernel32" ( _
    ByVal hFile As LongPtr, _
    lpBuffer As Any, _
    ByVal nNumberOfBytesToRead As Long, _
    lpNumberOfBytesRead As Long, _
    ByVal lpOverlapped As LongPtr) As Long

Private Declare PtrSafe Function CloseHandle Lib "kernel32" ( _
    ByVal hObject As LongPtr) As Long

Private Declare PtrSafe Function GetCurrentProcessId Lib "kernel32" () As Long

Private Declare PtrSafe Function GetTimeZoneInformation Lib "kernel32" (lpTimeZoneInformation As TIME_ZONE_INFORMATION) As Long

Private Type SYSTEMTIME
    wYear As Integer
    wMonth As Integer
    wDayOfWeek As Integer
    wDay As Integer
    wHour As Integer
    wMinute As Integer
    wSecond As Integer
    wMilliseconds As Integer
End Type

Private Type TIME_ZONE_INFORMATION
    Bias As Long
    StandardName(0 To 31) As Integer
    StandardDate As SYSTEMTIME
    StandardBias As Long
    DaylightName(0 To 31) As Integer
    DaylightDate As SYSTEMTIME
    DaylightBias As Long
End Type

Private Const GENERIC_READ As Long = &H80000000
Private Const GENERIC_WRITE As Long = &H40000000
Private Const OPEN_EXISTING As Long = 3
Private Const INVALID_HANDLE_VALUE As LongPtr = -1
Private Const TIME_ZONE_ID_DAYLIGHT As Long = 2

Private hPipe As LongPtr
Private mClientId As String
Private mConnected As Boolean

Public Type DiscordButton
    Label As String
    Url As String
End Type

Public Type DiscordTimestamps
    StartTime As Double
    EndTime As Double
End Type

Public Type DiscordAssets
    LargeImage As String
    LargeText As String
    SmallImage As String
    SmallText As String
End Type

Public Type DiscordParty
    PartyId As String
    CurrentSize As Long
    MaxSize As Long
End Type

Public Type DiscordSecrets
    JoinSecret As String
    SpectateSecret As String
    MatchSecret As String
End Type

Public Type DiscordActivity
    state As String
    Details As String
    Timestamps As DiscordTimestamps
    Assets As DiscordAssets
    Party As DiscordParty
    Secrets As DiscordSecrets
    Buttons(0 To 1) As DiscordButton
    Instance As Boolean
End Type

Public Function DiscordConnect(ClientId As String) As Boolean
    Dim i As Integer
    Dim pipeName As String
    
    mClientId = ClientId
    mConnected = False
    
    For i = 0 To 9
        pipeName = "\\.\pipe\discord-ipc-" & i
        hPipe = CreateFile(pipeName, GENERIC_READ Or GENERIC_WRITE, 0, 0, OPEN_EXISTING, 0, 0)
        If hPipe <> INVALID_HANDLE_VALUE Then Exit For
    Next i
    
    If hPipe = INVALID_HANDLE_VALUE Then
        DiscordConnect = False
        Exit Function
    End If
    
    Dim json As String
    json = "{""v"":1,""client_id"":""" & mClientId & """}"
    SendPacket 0, json
    ReadResponse
    
    mConnected = True
    DiscordConnect = True
End Function

Public Function DiscordIsConnected() As Boolean
    DiscordIsConnected = mConnected
End Function

Private Function StringToUTF8(text As String) As Byte()
    Dim i As Long
    Dim c As Long
    Dim utf8 As String
    
    For i = 1 To Len(text)
        c = AscW(Mid(text, i, 1))
        If c < 128 Then
            utf8 = utf8 & Chr(c)
        ElseIf c < 2048 Then
            utf8 = utf8 & Chr(192 Or (c \ 64))
            utf8 = utf8 & Chr(128 Or (c And 63))
        Else
            utf8 = utf8 & Chr(224 Or (c \ 4096))
            utf8 = utf8 & Chr(128 Or ((c \ 64) And 63))
            utf8 = utf8 & Chr(128 Or (c And 63))
        End If
    Next i
    
    StringToUTF8 = StrConv(utf8, vbFromUnicode)
End Function

Private Sub SendPacket(opcode As Long, data As String)
    Dim header(0 To 7) As Byte
    Dim dataBytes() As Byte
    Dim bytesWritten As Long
    Dim dataLen As Long
    
    dataBytes = StringToUTF8(data)
    dataLen = UBound(dataBytes) + 1
    
    header(0) = opcode And &HFF
    header(1) = (opcode \ &H100) And &HFF
    header(2) = (opcode \ &H10000) And &HFF
    header(3) = (opcode \ &H1000000) And &HFF
    header(4) = dataLen And &HFF
    header(5) = (dataLen \ &H100) And &HFF
    header(6) = (dataLen \ &H10000) And &HFF
    header(7) = (dataLen \ &H1000000) And &HFF
    
    WriteFile hPipe, header(0), 8, bytesWritten, 0
    WriteFile hPipe, dataBytes(0), dataLen, bytesWritten, 0
End Sub

Private Function ReadResponse() As String
    Dim header(0 To 7) As Byte
    Dim bytesRead As Long
    Dim dataLen As Long
    Dim dataBytes() As Byte
    
    ReadFile hPipe, header(0), 8, bytesRead, 0
    dataLen = header(4) + header(5) * &H100& + header(6) * &H10000 + header(7) * &H1000000
    
    If dataLen > 0 And dataLen < 65536 Then
        ReDim dataBytes(0 To dataLen - 1)
        ReadFile hPipe, dataBytes(0), dataLen, bytesRead, 0
        ReadResponse = dataBytes
    End If
End Function

Private Function GenerateNonce() As String
    Dim i As Integer
    Dim chars As String
    Randomize
    chars = "0123456789abcdef"
    For i = 1 To 32
        GenerateNonce = GenerateNonce & Mid(chars, Int(Rnd * 16) + 1, 1)
    Next i
End Function

Private Function EscapeJson(text As String) As String
    Dim result As String
    result = Replace(text, "\", "\\")
    result = Replace(result, """", "\""")
    result = Replace(result, vbCr, "\r")
    result = Replace(result, vbLf, "\n")
    result = Replace(result, vbTab, "\t")
    EscapeJson = result
End Function

Private Function UnixTimestamp(Optional dt As Date) As Double
    Dim tzi As TIME_ZONE_INFORMATION
    Dim result As Long
    Dim totalBias As Long
    
    result = GetTimeZoneInformation(tzi)
    totalBias = tzi.Bias
    
    If result = TIME_ZONE_ID_DAYLIGHT Then
        totalBias = totalBias + tzi.DaylightBias
    End If
    
    If dt = 0 Then dt = Now
    
    UnixTimestamp = DateDiff("s", #1/1/1970#, DateAdd("n", totalBias, dt))
End Function

Public Sub DiscordSetActivity(Activity As DiscordActivity)
    If hPipe = 0 Or hPipe = INVALID_HANDLE_VALUE Or Not mConnected Then Exit Sub
    
    Dim json As String
    Dim activityJson As String
    Dim nonce As String
    Dim hasContent As Boolean
    
    nonce = GenerateNonce()
    activityJson = ""
    hasContent = False
    
    If Activity.state <> "" Then
        activityJson = activityJson & """state"":""" & EscapeJson(Activity.state) & """"
        hasContent = True
    End If
    
    If Activity.Details <> "" Then
        If hasContent Then activityJson = activityJson & ","
        activityJson = activityJson & """details"":""" & EscapeJson(Activity.Details) & """"
        hasContent = True
    End If
    
    If Activity.Timestamps.StartTime <> 0 Or Activity.Timestamps.EndTime <> 0 Then
        If hasContent Then activityJson = activityJson & ","
        activityJson = activityJson & """timestamps"":{"
        Dim tsContent As Boolean
        tsContent = False
        If Activity.Timestamps.StartTime <> 0 Then
            activityJson = activityJson & """start"":" & format(Activity.Timestamps.StartTime, "0")
            tsContent = True
        End If
        If Activity.Timestamps.EndTime <> 0 Then
            If tsContent Then activityJson = activityJson & ","
            activityJson = activityJson & """end"":" & format(Activity.Timestamps.EndTime, "0")
        End If
        activityJson = activityJson & "}"
        hasContent = True
    End If
    
    If Activity.Assets.LargeImage <> "" Or Activity.Assets.SmallImage <> "" Then
        If hasContent Then activityJson = activityJson & ","
        activityJson = activityJson & """assets"":{"
        Dim assetContent As Boolean
        assetContent = False
        If Activity.Assets.LargeImage <> "" Then
            activityJson = activityJson & """large_image"":""" & EscapeJson(Activity.Assets.LargeImage) & """"
            assetContent = True
        End If
        If Activity.Assets.LargeText <> "" Then
            If assetContent Then activityJson = activityJson & ","
            activityJson = activityJson & """large_text"":""" & EscapeJson(Activity.Assets.LargeText) & """"
            assetContent = True
        End If
        If Activity.Assets.SmallImage <> "" Then
            If assetContent Then activityJson = activityJson & ","
            activityJson = activityJson & """small_image"":""" & EscapeJson(Activity.Assets.SmallImage) & """"
            assetContent = True
        End If
        If Activity.Assets.SmallText <> "" Then
            If assetContent Then activityJson = activityJson & ","
            activityJson = activityJson & """small_text"":""" & EscapeJson(Activity.Assets.SmallText) & """"
        End If
        activityJson = activityJson & "}"
        hasContent = True
    End If
    
    If Activity.Party.PartyId <> "" Then
        If hasContent Then activityJson = activityJson & ","
        activityJson = activityJson & """party"":{""id"":""" & EscapeJson(Activity.Party.PartyId) & """"
        If Activity.Party.CurrentSize > 0 And Activity.Party.MaxSize > 0 Then
            activityJson = activityJson & ",""size"":[" & Activity.Party.CurrentSize & "," & Activity.Party.MaxSize & "]"
        End If
        activityJson = activityJson & "}"
        hasContent = True
    End If
    
    If Activity.Secrets.JoinSecret <> "" Or Activity.Secrets.SpectateSecret <> "" Or Activity.Secrets.MatchSecret <> "" Then
        If hasContent Then activityJson = activityJson & ","
        activityJson = activityJson & """secrets"":{"
        Dim secretContent As Boolean
        secretContent = False
        If Activity.Secrets.JoinSecret <> "" Then
            activityJson = activityJson & """join"":""" & EscapeJson(Activity.Secrets.JoinSecret) & """"
            secretContent = True
        End If
        If Activity.Secrets.SpectateSecret <> "" Then
            If secretContent Then activityJson = activityJson & ","
            activityJson = activityJson & """spectate"":""" & EscapeJson(Activity.Secrets.SpectateSecret) & """"
            secretContent = True
        End If
        If Activity.Secrets.MatchSecret <> "" Then
            If secretContent Then activityJson = activityJson & ","
            activityJson = activityJson & """match"":""" & EscapeJson(Activity.Secrets.MatchSecret) & """"
        End If
        activityJson = activityJson & "}"
        hasContent = True
    End If
    
    If Activity.Buttons(0).Label <> "" Or Activity.Buttons(1).Label <> "" Then
        If hasContent Then activityJson = activityJson & ","
        activityJson = activityJson & """buttons"":["
        Dim btnContent As Boolean
        btnContent = False
        If Activity.Buttons(0).Label <> "" And Activity.Buttons(0).Url <> "" Then
            activityJson = activityJson & "{""label"":""" & EscapeJson(Activity.Buttons(0).Label) & """,""url"":""" & EscapeJson(Activity.Buttons(0).Url) & """}"
            btnContent = True
        End If
        If Activity.Buttons(1).Label <> "" And Activity.Buttons(1).Url <> "" Then
            If btnContent Then activityJson = activityJson & ","
            activityJson = activityJson & "{""label"":""" & EscapeJson(Activity.Buttons(1).Label) & """,""url"":""" & EscapeJson(Activity.Buttons(1).Url) & """}"
        End If
        activityJson = activityJson & "]"
        hasContent = True
    End If
    
    If Activity.Instance Then
        If hasContent Then activityJson = activityJson & ","
        activityJson = activityJson & """instance"":true"
    End If
    
    json = "{""cmd"":""SET_ACTIVITY"",""args"":{""pid"":" & GetCurrentProcessId() & ",""activity"":{" & activityJson & "}},""nonce"":""" & nonce & """}"
    
    SendPacket 1, json
    ReadResponse
End Sub

Public Sub DiscordSetActivitySimple(state As String, Details As String, Optional LargeImage As String = "", Optional LargeText As String = "", Optional SmallImage As String = "", Optional SmallText As String = "", Optional Button1Label As String = "", Optional Button1Url As String = "", Optional Button2Label As String = "", Optional Button2Url As String = "", Optional UseTimestamp As Boolean = True)
    If hPipe = 0 Or hPipe = INVALID_HANDLE_VALUE Or Not mConnected Then Exit Sub
    
    Dim act As DiscordActivity
    
    act.state = state
    act.Details = Details
    act.Assets.LargeImage = LargeImage
    act.Assets.LargeText = LargeText
    act.Assets.SmallImage = SmallImage
    act.Assets.SmallText = SmallText
    act.Buttons(0).Label = Button1Label
    act.Buttons(0).Url = Button1Url
    act.Buttons(1).Label = Button2Label
    act.Buttons(1).Url = Button2Url
    
    If UseTimestamp Then
        act.Timestamps.StartTime = UnixTimestamp()
    End If
    
    DiscordSetActivity act
End Sub

Public Sub DiscordClearActivity()
    If hPipe = 0 Or hPipe = INVALID_HANDLE_VALUE Or Not mConnected Then Exit Sub
    
    Dim json As String
    Dim nonce As String
    nonce = GenerateNonce()
    json = "{""cmd"":""SET_ACTIVITY"",""args"":{""pid"":" & GetCurrentProcessId() & ",""activity"":null},""nonce"":""" & nonce & """}"
    SendPacket 1, json
    ReadResponse
End Sub

Public Sub DiscordDisconnect()
    If hPipe <> 0 And hPipe <> INVALID_HANDLE_VALUE Then
        CloseHandle hPipe
        hPipe = 0
        mConnected = False
    End If
End Sub

Public Function DiscordGetUnixTimestamp(Optional dt As Date) As Double
    DiscordGetUnixTimestamp = UnixTimestamp(dt)
End Function
