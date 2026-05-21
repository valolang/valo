Attribute VB_Name = "BetterVB"
'     ____       __  __           _    ______
'    / __ )___  / /_/ /____  ____| |  / / __ )
'   / __  / _ \/ __/ __/ _ \/ ___/ | / / __  |
'  / /_/ /  __/ /_/ /_/  __/ /   | |/ / /_/ /
' /_____/\___/\__/\__/\___/_/    |___/_____/
' by: Erick / Arfur / Erico / Victor Gabriel

' --> Participações especiais: Danni / Gabb / iDavi / Ueslei

    Option Explicit

'  _    _____    ____  _______ _    _________________
' | |  / /   |  / __ \/  _/   | |  / / ____/  _/ ___/
' | | / / /| | / /_/ // // /| | | / / __/  / / \__ \
' | |/ / ___ |/ _  _// // ___ | |/ / /____/ / ___/ /
' |___/_/  |_/_/ |_/___/_/  |_|___/_____/___//____/

#If VBA7 Then
    Public Declare PtrSafe Function sndPlaySound32 Lib "winmm.dll" Alias "sndPlaySoundA" (ByVal lpszSoundName As String, ByVal uFlags As Long) As Long
    Public Declare PtrSafe Function GetAsyncKeyState Lib "user32" (ByVal vKey As Long) As Integer
    Public Declare PtrSafe Function GetTickCount Lib "kernel32" () As Long
    Public Declare PtrSafe Function URLDownloadToFile Lib "urlmon" Alias "URLDownloadToFileA" (ByVal pCaller As Long, ByVal szURL As String, ByVal szFileName As String, ByVal dwReserved As Long, ByVal lpfnCB As Long) As Long
    Private Declare PtrSafe Function GetCursorPos Lib "user32" (lpPoint As POINTAPI) As LongPtr
    Private Declare PtrSafe Function LoadCursor Lib "user32" Alias "LoadCursorA" (ByVal hInstance As Long, ByVal lpCursorName As Long) As Long
    Private Declare PtrSafe Function SetCursor Lib "user32" (ByVal hCursor As Long) As Long
    Private Declare PtrSafe Function SetCursorPos Lib "user32" (ByVal x As Long, ByVal Y As Long) As Long
    Private Declare PtrSafe Function GetDeviceCaps Lib "gdi32" (ByVal hdc As LongPtr, ByVal nIndex As Long) As Long
    Private Declare PtrSafe Function GetDC Lib "user32" (ByVal hWnd As LongPtr) As LongPtr
    Private Declare PtrSafe Function ReleaseDC Lib "user32" (ByVal hWnd As LongPtr, ByVal hdc As LongPtr) As Long
    Private Declare PtrSafe Function FindWindow Lib "user32" Alias "FindWindowA" (ByVal lpClassName As String, ByVal lpWindowName As String) As LongPtr
    Private Declare PtrSafe Function MapWindowPoints Lib "user32" (ByVal hwndFrom As LongPtr, ByVal hwndTo As LongPtr, lppt As Any, ByVal cPoints As Long) As Long
    Private Declare PtrSafe Function GetDesktopWindow Lib "user32" () As LongPtr
    Private Declare PtrSafe Function GetWindowRect Lib "user32" (ByVal hWnd As LongPtr, lpRect As RECT) As LongPtr
#End If

' BetterVB =----------------------------------------------------------------------------------------------------------------------------------------=
Public cSlide As Slide, GameDataFolder As String, CustomPath As String, FPS As Long, DeltaTime As Double, DeltaTime0 As Double
Private PressedKeys(255) As Boolean, LastFps As Double, LastFrames As Double, FrameCount As Integer, FileSystem As Object, pReq As Object
Private CachedTimerShape As Shape
Private RoundMoveX As Double, RoundMoveY As Double, Angle As Double

Private Const SND_ASYNC = &H1
Public Const PI As Double = 3.14159265358979, e As Double = 2.71828182846, PI2 As Double = 1.57079632679, TAU As Double = 6.28318530718
Public Const ASCII_LCASE As String = "abcdefghijklmnopqrstuvwxyz", ASCII_UCASE As String = "ABCDEFGHIJKLMNOPQRSTUVWXYZ", ASCII_LETTERS As String = ASCII_UCASE & ASCII_LCASE, DIGITS As String = "0123456789", PUNCTUATION As String = "!""#$%&'()*+,-./:;<=>?@[\]^_`{|}~", PRINTABLE As String = ASCII_LETTERS & DIGITS & PUNCTUATION & " ", HEXDIGITS As String = "0123456789abcdefABCDEF", OCTDIGITS As String = "01234567"
Enum FETypeJ: DeleteShp = 0: Rename = 1: Visible = 2: Text = 3: Image = 4: Rotation = 5: End Enum
Enum InSlidePr: InSlide = 0: InPresentation = 1: End Enum
Enum EqualOrDiferent: Equal = 0: Diferent = 1: End Enum
Enum Options: Lower = 1: Upper = 2: Normal = 0: End Enum
Public Type CRGB: r As Byte: G As Byte: b As Byte: End Type
Enum SideType: shpCenter = 0: shpleft = 1: shpRight = 2: shpTop = 3: shpBottom = 4: shptopleft = 5: shpTopRight = 6: shpBottomLeft = 7: shpBottomRight = 8: End Enum

' SpriteSheets Pica do Xeno =-----------------------------------------------------------------------------------------------------------------------=
Private Activated As New Dictionary, Indexed As New Dictionary, Sheets As New Dictionary

' DynZ =--------------------------------------------------------------------------------------------------------------------------------------------=
Private pEntities As New Dictionary, pObjects As New Dictionary, i As Long, Item As Shape, Ent As Dictionary, EntShp As Shape, Obj As Dictionary, EntityX As Single, EntityY As Single

' Colisao Pica do Xeno =----------------------------------------------------------------------------------------------------------------------------=
Enum CollisionDirection: CollisionDirectionDown = 0: CollisionDirectionLeft = 1: CollisionDirectionRight = 2: CollisionDirectionUp = 3: End Enum
Private Handler As Object, Objects As New Dictionary
Private Const SpaceColl As Double = 0.001

' CursorAPI  =--------------------------------------------------------------------------------------------------------------------------------------=
Private Const LOGPIXELSX As Long = 88
Private mPoint As POINTAPI
Private Type POINTAPI: x As Long: Y As Long: End Type
Private Type RECT: lLeft As Long: lTop As Long: lRight As Long: lBottom As Long: End Type

' JsonVBA =-----------------------------------------------------------------------------------------------------------------------------------------=
Private ind As Long, ws As Integer, lb As LineBreaks
Private Type json_options: AllowUnquotedKeys As Boolean: UseEscapeChars As Boolean: IgnoreUndefinedExpr As Boolean: DefinedExpr As New List: End Type
Public Enum LineBreaks: NeverBreak = 0: AlwaysBreak = 1: BreakOnMain = 2: End Enum
Public JsonOptions As json_options

Private Type IntervalTimer
    key As String
    StartTime As Double
    Once As Boolean
    Done As Boolean
End Type

Private Timers() As IntervalTimer
Private TimerCount As Long
Private Initialized As Boolean

'    ____  _   ____    _____   ________
'   / __ \/ | / / /   /  _/ | / / ____/
'  / / / /  |/ / /    / //  |/ / __/
' / /_/ / /|  / /____/ // /|  / /___
' \____/_/ |_/_____/___/_/ |_/_____/

Public Function httpget(Url As String) As String
    Dim objHTTP As Object
    Set objHTTP = CreateObject("MSXML2.ServerXMLHTTP")
        objHTTP.Open "GET", Url, False
        objHTTP.setRequestHeader "User-Agent", "Mozilla/4.0 (compatible; MSIE 6.0; Windows NT 5.0)"
        objHTTP.Send ""
    httpget = objHTTP.ResponseText
End Function

Public Function httppost(Url As String, content As String) As String
    Dim objHTTP As Object
    Set objHTTP = CreateObject("MSXML2.ServerXMLHTTP")
        objHTTP.Open "POST", Url, False
        objHTTP.setRequestHeader "User-Agent", "Mozilla/4.0 (compatible; MSIE 6.0; Windows NT 5.0)"
        objHTTP.Send content
    httppost = objHTTP.ResponseText
End Function

Function MediafireDownload(Link As String, Path As String, Optional AvisoBox As Boolean)
    If Dir(Path, vbDirectory) = "" Then
        URLDownloadToFile 0, GetMediafireUrl(Link), Path, 0, 0
    Else
        Kill Path
        URLDownloadToFile 0, GetMediafireUrl(Link), Path, 0, 0
    End If
    If AvisoBox Then InputBox "Download Completado com sucesso", "erckCore - MediaFire Download", Path
End Function

Private Function GetMediafireUrl(Link As String) As String
    Dim oHtml As New HTMLDocument, htmlstr
    htmlstr = httpget(Link)
    oHtml.Body.innerHTML = htmlstr
    GetMediafireUrl = oHtml.querySelectorAll("a")(7).href
End Function

Private Sub EnsureInitialized()
    If Not Initialized Then
        ReDim Timers(1 To 8)
        TimerCount = 0
        Initialized = True
    End If
End Sub

Private Function BinarySearch(key As String, ByRef found As Boolean) As Long
    Dim low As Long, high As Long, mid As Long, cmp As Long
    low = 1: high = TimerCount
    Do While low <= high
        mid = (low + high) \ 2
        cmp = StrComp(key, Timers(mid).key, vbBinaryCompare)
        If cmp = 0 Then
            found = True: BinarySearch = mid: Exit Function
        ElseIf cmp < 0 Then
            high = mid - 1
        Else
            low = mid + 1
        End If
    Loop
    found = False: BinarySearch = low
End Function

Public Sub ClearInterval(key As String)
    Dim idx As Long, found As Boolean, i As Long
    idx = BinarySearch(key, found)
    If found Then
        For i = idx To TimerCount - 1
            Timers(i) = Timers(i + 1)
        Next i
        TimerCount = TimerCount - 1
    End If
End Sub

Public Sub ClearIntervals()
    If Initialized Then
        TimerCount = 0
        Initialized = False
        Erase Timers
    End If
End Sub

Public Function Wait(Seconds As Double, key As String, Optional Once As Boolean = False) As Boolean
    Dim idx As Long, found As Boolean, insertPos As Long
    Dim currentTime As Double

    EnsureInitialized
    currentTime = Timer
    idx = BinarySearch(key, found)

    If Not found Then
        insertPos = idx
        TimerCount = TimerCount + 1
        If TimerCount > UBound(Timers) Then ReDim Preserve Timers(1 To UBound(Timers) * 2)
        If insertPos <= TimerCount - 1 Then
            Dim j As Long
            For j = TimerCount To insertPos + 1 Step -1
                Timers(j) = Timers(j - 1)
            Next j
        End If
        With Timers(insertPos)
            .key = key
            .StartTime = currentTime
            .Once = Once
            .Done = False
        End With
        idx = insertPos
    ElseIf Timers(idx).Once And Timers(idx).Done Then
        Exit Function
    End If

    With Timers(idx)
        If (currentTime - .StartTime) >= Seconds Then
            Wait = True
            If .Once Then
                .Done = True
            Else
                .StartTime = currentTime
            End If
        End If
    End With
End Function

'    _____ __               __             __
'   / ___// /_  ____  _____/ /________  __/ /______
'   \__ \/ __ \/ __ \/ ___/ __/ ___/ / / / __/ ___/
'  ___/ / / / / /_/ / /  / /_/ /__/ /_/ / /_(__  )
' /____/_/ /_/\____/_/   \__/\___/\__,_/\__/____/

Public Function DocPath() As String
    Dim oShell As Object: Set oShell = CreateObject("WScript.Shell")
    DocPath = oShell.SpecialFolders("MyDocuments")
End Function

Public Function NewFolder(Path As String)
    If Dir(Path, vbDirectory) = "" Then MkDir Path
End Function

Public Function GoEx(ByVal SlideApr As Long, MacroName As String)
    GoToSlide SlideApr
    Application.Run ActivePresentation.Slides(SlideApr).Name & "." & MacroName
End Function

Public Function AprPath() As String
    AprPath = ActivePresentation.Path
End Function

Public Function AppDataPath() As String
    AppDataPath = Environ("AppData")
End Function

Public Function PlaySound(SoundFile As String)
    sndPlaySound32 SoundFile, SND_ASYNC
End Function

Public Function sp(PStr As String) As String
    PStr = Replace$(PStr, "/", "\")
    Select Case Left$(PStr, 2)
        Case ".\": sp = AprPath & "\" & Mid$(PStr, 3)
        Case ",\": sp = DocPath & "\" & Mid$(PStr, 3)
        Case ";\": sp = AppDataPath & "\" & Mid$(PStr, 3)
        Case ":\": sp = CustomPath & "\" & Mid$(PStr, 3)
        Case Else: sp = PStr
    End Select
End Function

Public Property Let ChangeText(Shape As Shape, Text As String)
    If Shape.TextEffect.Text <> Text Then Shape.TextEffect.Text = Text
End Property

Public Function CSShape(Shape As String) As Shape
    Set CSShape = cSlide.Shapes(Shape)
End Function

Public Function ShapeToMouse(Shape As Shape)
    Shape.Left = GetCursorX - (Shape.Width / 2)
    Shape.Top = GetCursorY - (Shape.Height / 2)
End Function

'    _____ ___     __     __  ____  _ __
'   / ___// (_)___/ /__  / / / / /_(_) /____
'   \__ \/ / / __  / _ \/ / / / __/ / / ___/
'  ___/ / / / /_/ /  __/ /_/ / /_/ / (__  )
' /____/_/_/\____/\___/\____/\__/_/_/____/

Public Function GoToSlide(ByVal Slide As Long, Optional ResetSlide As MsoTriState = msoTrue)
    ActivePresentation.SlideShowWindow.View.GoToSlide Slide, ResetSlide
End Function

Public Function GoToNext()
    ActivePresentation.SlideShowWindow.View.Next
End Function

Public Function GoToPrev()
    ActivePresentation.SlideShowWindow.View.Previous
End Function

Public Function CurrentSlide() As Long
    CurrentSlide = ActivePresentation.SlideShowWindow.View.CurrentShowPosition
End Function

Public Function LastViewedSlideIndex() As Long
    LastViewedSlideIndex = ActivePresentation.SlideShowWindow.View.LastSlideViewed
End Function

Public Function LastViewedSlide() As Slide
    Set LastViewedSlide = ActivePresentation.Slides(ActivePresentation.SlideShowWindow.View.LastSlideViewed)
End Function

Public Function FirstSlide()
    ActivePresentation.SlideShowWindow.View.First
End Function

Public Function LastSlide()
    ActivePresentation.SlideShowWindow.View.Last
End Function

Public Function ResetSlideTime()
    ActivePresentation.SlideShowWindow.View.ResetSlideTime
End Function

Public Function ExitSlideShow()
    ActivePresentation.SlideShowWindow.View.Exit
End Function

Public Function SlideWidth() As Single
    SlideWidth = ActivePresentation.PageSetup.SlideWidth
End Function

Public Function SlideHeight() As Single
    SlideHeight = ActivePresentation.PageSetup.SlideHeight
End Function

'   _____ __       _             __  ____  _ __
'  / ___// /______(_)___  ____ _/ / / / /_(_) /____
'  \__ \/ __/ ___/ / __ \/ __ \/ / / / __/ / / ___/
' ___/ / /_/ /  / / / / / /_/ / /_/ / /_/ / (__  )
'/____/\__/_/  /_/_/ /_/\__, /\____/\__/_/_/____/
'                      /____/

Public Function Includes(ByVal str As String, ByVal SearchString As String, Optional ByVal StartPos As Long = 1, Optional Compare As VbCompareMethod = vbBinaryCompare) As Boolean
    Includes = InStr(StartPos, str, SearchString, Compare)
End Function

Public Function StartsWith(ByVal str As String, ByVal SearchString As String, Optional ByVal Position As Long = 1) As Boolean
    StartsWith = mid(str, Position, Len(SearchString)) = SearchString
End Function

Public Function EndsWith(ByVal str As String, ByVal SearchString As String, Optional ByVal EndPosition As Long) As Boolean
    If EndPosition = 0 Then EndPosition = Len(str)
    EndsWith = mid(str, EndPosition - Len(SearchString) + 1, Len(SearchString)) = SearchString
End Function

Public Function Concat(ByRef String1 As String, ByVal String2 As String) As Long
    String1 = String1 & String2
    Concat = Len(String1)
End Function

Function StrJoin(s As String, delimiter As String) As String
    Dim i As Long
    Dim res As String
    s = Replace(s, " ", "")
    For i = 1 To Len(s)
        res = res & mid(s, i, 1) & IIf(i < Len(s), delimiter, "")
    Next
    StrJoin = res
End Function

Public Function SwapCase(Text As String) As String
    Dim i%, p As String
    For i = 1 To Len(Text)
        p = p & IIf(IsUpper(mid(Text, i, 1)) = True, LCase(mid(Text, i, 1)), UCase(mid(Text, i, 1)))
    Next
    SwapCase = p
End Function

Public Function IsUpper(Text As String) As Boolean
    If Text = UCase(Text) Then IsUpper = True
End Function

Public Function IsLower(Text As String) As Boolean
    If Text = LCase(Text) Then IsLower = True
End Function

Public Function count(ByVal Text As String, ByVal TextToCount As String, Optional StartPosition As Long = 1, Optional ByVal Compare As VbCompareMethod = vbBinaryCompare) As Long
    Dim i%
    For i = 1 To Len(Text)
        If InStr(StartPosition, mid(Text, i, Len(TextToCount)), TextToCount, Compare) > 0 Then count = count + 1
    Next
End Function

Public Function IsPrintable(ByVal str As String) As Boolean
    Dim i As Long
    IsPrintable = True
    For i = 1 To Len(str)
        If Not Includes(PRINTABLE, mid(str, i, 1)) Then
            IsPrintable = False
            Exit Function
        End If
    Next
End Function

Public Function ToCharArray(ByVal Text As String)
    Dim temparr As Variant, i As Single
    temparr = Array()
    If Len(Text) <> 0 Then
        For i = 1 To Len(Text)
            ReDim Preserve temparr(UBound(temparr) + 1)
            temparr(UBound(temparr)) = mid(Text, i, 1)
        Next
        ToCharArray = temparr
    End If
End Function

Public Function Choice(ParamArray Var() As Variant)
    Choice = Var(RandomNum(LBound(Var), UBound(Var)))
End Function

Public Function TextOverflow(Shape As Shape, content As String, MultiLine As Boolean, Ellipsis As Boolean)
    
    Shape.TextFrame.TextRange.Text = content
    
    Dim Lines As TextRange
    Set Lines = Shape.TextFrame.TextRange.Lines
    
    If MultiLine Then
        Dim Height As Double: Height = Shape.Height - Shape.TextFrame.MarginBottom - Shape.TextFrame.MarginTop
        If Lines.BoundHeight > Height * 2 Then
            Dim LineCount As Integer: LineCount = 1
            Do While Lines.Lines(1, LineCount).BoundHeight < Height
                LineCount = LineCount + 1
            Loop
            Lines.Lines(LineCount, Lines.count).Delete
        ElseIf Lines.BoundHeight > Height Then
            Dim count As Integer: count = Lines.count
            Do While Lines.BoundHeight > Height
                count = count - 1
                Lines.Lines(count - 1, count).Delete
                If count = 2 Then Exit Do
            Loop
        Else
            Exit Function
        End If
    Else
        If Lines.count > 1 Then Lines.Lines(2, Lines.count).Delete Else Exit Function
    End If
    
    If Ellipsis Then
        
        Dim Line As TextRange
        Set Line = Lines.Lines(Lines.count, 1)
        
        content = Line.Text
        Line.Text = content & "..."
        
        Do While Line.count > 1
            If content = "" Then
                Line.Delete
                Exit Do
            End If
            content = Left(content, Len(content) - 1)
            Line.Text = content & "..."
        Loop
        
    End If
    
End Function

Public Function Translate(Text As String, Language As String) As String
    Dim req As Object
    Set req = CreateObject("MSXML2.ServerXMLHTTP")
    req.Open "GET", "https://uesleitradutorapi.uesleidev.repl.co/" & Text & "/" & Language, False
    req.Send ""
    Translate = req.ResponseText
    req.abort
End Function

'    __  ___      __  __    __  ____  _ __
'   /  |/  /___ _/ /_/ /_  / / / / /_(_) /____
'  / /|_/ / __ \/ __/ __ \/ / / / __/ / / ___/
' / /  / / /_/ / /_/ / / / /_/ / /_/ / (__  )
'/_/  /_/\__,_/\__/_/ /_/\____/\__/_/_/____/

Public Function DegToRad(ByVal Deg As Double) As Double
    DegToRad = Deg * PI / 180
End Function

Public Function RadToDeg(ByVal Rad As Double) As Double
    RadToDeg = Rad * 180 / PI
End Function

Public Function GetDistance(ByVal X1 As Double, ByVal Y1 As Double, ByVal X2 As Double, ByVal Y2 As Double) As Double
    Dim dx As Double, dy As Double
    dx = X1 - X2
    dy = Y1 - Y2
    GetDistance = Sqr((dx * dx) + (dy * dy))
End Function

Public Function GetDistance2(ByVal X1 As Double, ByVal Y1 As Double, ByVal X2 As Double, ByVal Y2 As Double) As Double
    Dim dx As Double, dy As Double
    dx = X1 - X2
    dy = Y1 - Y2
    GetDistance2 = (dx * dx) + (dy * dy)
End Function

Public Function GetDistance3(ByVal X1#, ByVal X2#, ByVal Y1#, ByVal Y2#, ByVal Z1#, ByVal Z2#, Optional ByVal Sqrt As Boolean = True) As Double
    GetDistance3 = IIf(Sqrt, Sqr((X2 - X1) ^ 2 + (Y2 - Y1) ^ 2 + (Z2 - Z1) ^ 2), (X2 - X1) ^ 2 + (Y2 - Y1) ^ 2 + (Z2 - Z1) ^ 2)
End Function

Public Function RandomNum(Optional ByVal Minimum As Single, Optional ByVal Maximum As Single = 1, Optional RandomizeNumber As Variant, Optional RndNumber As Single) As Single
    If IsMissing(RandomizeNumber) Then
        Randomize
    Else
        Randomize RandomizeNumber
    End If
    RandomNum = (Maximum - Minimum) * Rnd + Minimum
End Function

Public Function RandomInt(Optional ByVal Minimum As Single, Optional ByVal Maximum As Single = 1, Optional RandomizeNumber As Variant, Optional RndNumber As Single)
    RandomInt = Round(RandomNum(Minimum, Maximum, RandomizeNumber, RndNumber))
End Function

Public Function Trunc(ByVal x#) As Long
    Trunc = IIf(x > 0, Int(x), Int(x * -1) * -1)
End Function

Public Function Min(ParamArray x() As Variant) As Double
    Dim i%
    For i = LBound(x) To UBound(x)
        If i = 0 Or x(i) < Min Then Min = x(i)
    Next
End Function

Public Function Max(ParamArray x() As Variant) As Double
    Dim i%
    For i = LBound(x) To UBound(x)
        If i = 0 Or x(i) > Max Then Max = x(i)
    Next
End Function

Public Function Floor(ByVal Number As Double) As Long
    Floor = Int(Number)
End Function

Public Function Ceil(ByVal x#) As Long
    Ceil = IIf(x = Int(x), x, Int(x + 1))
End Function

Public Function Atn2(x#, Y#) As Double
    If x > 0 Then
        Atn2 = Atn(Y / x)
    ElseIf x < 0 Then
        Atn2 = Atn(Y / x) + PI * Sgn(Y)
        If Y = 0 Then Atn2 = Atn2 + PI
    Else
        Atn2 = PI / 2 * Sgn(Y)
    End If
End Function

Public Function AddWithMax(ByVal ToAdd As Double, ByVal Plus As Double, Optional ByVal Max As Double = 0) As Double
    Dim Result As Double
    Result = ToAdd + Plus
    If Result >= Max Then
        AddWithMax = Max
    Else
        AddWithMax = Result
    End If
End Function

Public Function RemoveWithMin(ByVal ToRem As Double, ByVal Minus As Double, Optional ByVal Min As Double = 0) As Double
    Dim Result As Double
    Result = ToRem - Minus
    If Result <= Min Then
        RemoveWithMin = Min
    Else
        RemoveWithMin = Result
    End If
End Function

Public Function Root(ByVal x#, Optional ByVal Y As Double = 2) As Double
    Root = Abs(x) ^ (1 / Y)
End Function

Public Function Hypot(b#, c#) As Double
    Hypot = Sqr(b ^ 2 + c ^ 2)
End Function

Function Evaluate(ByVal String1 As String) As Double
    On Error Resume Next
    Dim Excel As Object: Set Excel = CreateObject("Excel.Application")
    Evaluate = Excel.Evaluate(String1)
End Function

Public Function GCD(ByVal a As Long, ByVal b As Long) As Long
    Dim remainder As Long
    If a = 0 Or b = 0 Then Exit Function
    Do
      remainder = Abs(a) Mod Abs(b)
      a = Abs(b)
      b = remainder
    Loop Until remainder = 0
    GCD = a
End Function

Public Function LCM(ByVal a As Long, ByVal b As Long) As Long
    If a = 0 Or b = 0 Then Exit Function
    LCM = (Abs(a) * Abs(b)) \ GCD(a, b)
End Function

Public Function Fact(ByVal n As Long, Optional ByVal StepValue As Long = 1) As LongPtr
    Fact = 1
    For n = n To 1 Step -Abs(StepValue)
        Fact = Fact * n
    Next
End Function

Public Function Fibonacci(ByVal n As Long) As LongPtr
    If n <= 0 Then Exit Function
    Fibonacci = IIf(n = 1, 1, Fibonacci(n - 1) + Fibonacci(n - 2))
End Function

Public Function Mean(ParamArray x() As Variant) As Double
    Dim i%
    For i = LBound(x) To UBound(x)
        Mean = Mean + x(i)
    Next
    Mean = Mean / (UBound(x) + 1)
End Function

Public Function Median(ParamArray x() As Variant) As Double
    Median = x(0)
    If UBound(x) = 0 Then Exit Function
    Median = IIf(UBound(x) Mod 2, (x(UBound(x) \ 2) + x(UBound(x) \ 2 + 1)) / 2, x(UBound(x) \ 2))
End Function

Public Function Variance(ByVal N1#, ByVal N2#) As Double
    Variance = (Mean(N1, N2) - N1) ^ 2 + (Mean(N1, N2) - N2) ^ 2
End Function

Public Function XMid(ByVal X1#, ByVal X2#) As Double
    XMid = (X1 + X2) / 2
End Function

Public Function YMid(ByVal Y1#, ByVal Y2#) As Double
    YMid = (Y1 + Y2) / 2
End Function

Public Function FindA(ByVal X1#, ByVal X2#, ByVal Y1#, ByVal Y2#) As Double
    If X1 = X2 Then Exit Function
    FindA = (Y1 - Y2) / (X1 - X2)
End Function

Public Function Lerp(ByVal X1#, ByVal X2#, ByVal Y1#, ByVal Y2#, ByVal x#) As Double
    If X1 = X2 Then Exit Function
    Lerp = Y1 + (x - X1) * (Y2 - Y1) / (X2 - X1)
End Function

Public Function LineLineIntersect(ByVal X1#, ByVal Y1#, ByVal X2#, ByVal Y2#, ByVal x3#, ByVal y3#, ByVal x4#, ByVal y4#)
    Dim x As Double, Y As Double
    If (X1 - X2) * (y3 - y4) = (Y1 - Y2) * (x3 - x4) Then Exit Function
    x = ((X1 * Y2 - Y1 * X2) * (x3 - x4) - (X1 - X2) * (x3 * y4 - y3 * x4)) / ((X1 - X2) * (y3 - y4) - (Y1 - Y2) * (x3 - x4))
    Y = ((X1 * Y2 - Y1 * X2) * (y3 - y4) - (Y1 - Y2) * (x3 * y4 - y3 * x4)) / ((X1 - X2) * (y3 - y4) - (Y1 - Y2) * (x3 - x4))
    LineLineIntersect = Array(x, Y)
End Function

Public Function Sec(ByVal x#) As Double
    Sec = 1 / Cos(x)
End Function

Public Function Cosec(ByVal x#) As Double
    Cosec = 1 / Sin(x)
End Function

Public Function Cotan(ByVal x#) As Double
    Cotan = 1 / Tan(x)
End Function

Public Function ASin(ByVal x#) As Double
    ASin = Atn(x / Root(-x * x + 1))
End Function

Public Function ACos(ByVal x#) As Double
    ACos = Atn(-x / Root(-x * x + 1)) + 2 * Atn(1)
End Function

Public Function ASec(ByVal x#) As Double
    ASec = Atn(x / Root(x * x - 1)) + Sgn((x) - 1) * (2 * Atn(1))
End Function

Public Function ACosec(ByVal x#) As Double
    ACosec = Atn(x / Root(x * x - 1)) + (Sgn(x) - 1) * (2 * Atn(1))
End Function

Public Function ACotan(ByVal x#) As Double
    ACotan = Atn(x) + 2 * Atn(1)
End Function

Public Function HSin(ByVal x#) As Double
    HSin = (Exp(x) - Exp(-x)) / 2
End Function

Public Function HCos(ByVal x#) As Double
    HCos = (Exp(x) + Exp(-x)) / 2
End Function

Public Function HTan(ByVal x#) As Double
    HTan = (Exp(x) - Exp(-x)) / (Exp(x) + Exp(-x))
End Function

Public Function HSec(ByVal x#) As Double
    HSec = 2 / (Exp(x) + Exp(-x))
End Function

Public Function HCosec(ByVal x#) As Double
    HCosec = 2 / (Exp(x) - Exp(-x))
End Function

Public Function HCotan(ByVal x#) As Double
    HCotan = (Exp(x) + Exp(-x)) / (Exp(x) - Exp(-x))
End Function

Public Function HASin(ByVal x#) As Double
    HASin = Log(x + Root(x * x + 1))
End Function

Public Function HACos(ByVal x#) As Double
    HACos = Log(x + Root(x * x - 1))
End Function

Public Function HATan(ByVal x#) As Double
    HATan = Log((1 + x) / (1 - x)) / 2
End Function

Public Function HASec(ByVal x#) As Double
    HASec = Log((Root(-x * x + 1) + 1) / x)
End Function

Public Function HACosec(ByVal x#) As Double
    HACosec = Log((Sgn(x) * Root(x * x + 1) + 1) / x)
End Function

Public Function HACotan(ByVal x#) As Double
    HACotan = Log((x + 1) / (x - 1)) / 2
End Function

'   ___                          ____  __
'   /   |  ______________ ___  __/ __ \/ /_  _______
'  / /| | / ___/ ___/ __ `/ / / / /_/ / / / / / ___/
' / ___ |/ /  / /  / /_/ / /_/ / ____/ / /_/ (__  )
'/_/  |_/_/  /_/   \__,_/\__, /_/   /_/\__,_/____/
'                       /____/

Public Function LengthOf(List As Variant) As Long
    If IsArray(List) Then LengthOf = UBound(List) + 1
End Function

Public Function MinOf(List As Variant) As Double
    Dim i%
    If IsArray(List) Then
        For i = LBound(List) To UBound(List)
            If IsNumeric(List(i)) Then
                If i = 0 Or List(i) < MinOf Then MinOf = List(i)
            End If
        Next
    End If
End Function

Public Function MaxOf(List As Variant) As Double
    Dim i%
    If IsArray(List) Then
        For i = LBound(List) To UBound(List)
            If IsNumeric(List(i)) Then
                If i = 0 Or List(i) > MaxOf Then MaxOf = List(i)
            End If
        Next
    End If
End Function

Public Function At(List As Variant, Optional ByVal Index As Long = 0, Optional ByVal ReturnIndex As Boolean = False)
    If IsArray(List) Then
        If UBound(List) + 1 > 0 Then
            If Index >= 0 And Index <= UBound(List) Then
                At = List(Index)
            ElseIf Index > UBound(List) Then
                At = List(UBound(List))
            ElseIf Index < 0 And Abs(Index) <= UBound(List) + 1 Then
                At = List((UBound(List) + 1) + Index)
            Else
                At = List(0)
            End If
        End If
        If ReturnIndex Then At = IndexOf(List, At)
    End If
End Function

Public Function Insert(List As Variant, Item As Variant, Optional ByVal Index As Long)
    Dim i As Long
    If IsArray(List) Then
        Index = At(List, Index, True)
        If UBound(List) < Index Or Index = 0 Then
            ReDim Preserve List(UBound(List) + 1)
            If IsObject(Item) Then
                Set List(UBound(List)) = Item
            Else
                List(UBound(List)) = Item
            End If
        ElseIf UBound(List) >= Index Then
            ReDim Preserve List(UBound(List) + 1)
            For i = UBound(List) - 1 To Index Step -1
                If IsObject(List(i)) Then
                    Set List(i + 1) = List(i)
                Else
                    List(i + 1) = List(i)
                End If
            Next
            If IsObject(Item) Then
                Set List(Index) = Item
            Else
                List(Index) = Item
            End If
        End If
    End If
End Function

Public Function RemoveArray(List As Variant, ByVal value)
    Dim i As Long, Index As Long
    If IsArray(List) And IncludesOf(List, value) Then
        Index = IndexOf(List, value)
        If UBound(List) = Index And UBound(List) + 1 > 1 Then
            ReDim Preserve List(UBound(List) - 1)
        ElseIf UBound(List) > Index Then
            For i = Index To UBound(List)
                If i <> UBound(List) Then
                    If IsObject(List(i + 1)) Then
                        Set List(i) = List(i + 1)
                    Else
                        List(i) = List(i + 1)
                    End If
                End If
            Next
            ReDim Preserve List(UBound(List) - 1)
        ElseIf UBound(List) >= Index And UBound(List) + 1 = 1 Then
            ReDim List(UBound(List) - UBound(List))
        Else
            Exit Function
        End If
    End If
End Function

Public Function Pop(List As Variant, Optional ByVal Index As Long)
    Dim i As Long
    If IsArray(List) Then
        If IsMissing(Index) = False Then Index = At(List, Index, True)
        If IsMissing(Index) Or Index > UBound(List) Or UBound(List) = Index And UBound(List) + 1 > 1 Then
            ReDim Preserve List(UBound(List) - 1)
        ElseIf UBound(List) > Index Then
            For i = Index To UBound(List)
                If i <> UBound(List) Then
                    If IsObject(List(i + 1)) Then
                        Set List(i) = List(i + 1)
                    Else
                        List(i) = List(i + 1)
                    End If
                End If
            Next
            ReDim Preserve List(UBound(List) - 1)
        ElseIf UBound(List) >= Index And UBound(List) + 1 = 1 Then
            ReDim List(UBound(List) - UBound(List))
        Else
            Exit Function
        End If
    End If
End Function

Public Function IncludesOf(List As Variant, Item As Variant) As Boolean
    Dim i As Long
    If IsArray(List) Then
        For i = 0 To UBound(List)
            If IsObject(Item) And IsObject(List(i)) Then
                If List(i) Is Item Then IncludesOf = True
            ElseIf Not IsObject(Item) And Not IsObject(List(i)) Then
                If List(i) = Item Then IncludesOf = True
            End If
        Next
    End If
End Function

Public Function IndexOf(List As Variant, Item As Variant) As Long
    Dim i As Long
    If IsArray(List) Then
        For i = 0 To UBound(List)
            If IsObject(Item) And IsObject(List(i)) Then
                If List(i) Is Item Then
                    IndexOf = i
                    Exit Function
                Else
                    IndexOf = -1
                End If
            ElseIf Not IsObject(Item) And Not IsObject(List(i)) Then
                If List(i) = Item Then
                    IndexOf = i
                    Exit Function
                Else
                    IndexOf = -1
                End If
            End If
        Next
    End If
End Function

Public Function CountOf(List As Variant, Item As Variant) As Long
    Dim i As Long
    If IsArray(List) Then
        For i = 0 To UBound(List)
            If IsObject(Item) And IsObject(List(i)) Then
                If List(i) Is Item Then CountOf = CountOf + 1
            ElseIf Not IsObject(Item) And Not IsObject(List(i)) Then
                If List(i) = Item Then CountOf = CountOf + 1
            End If
        Next
    End If
End Function

Public Function Reverse(List As Variant)
    Dim handlerlist As Variant, i As Long
    handlerlist = Array()
    If IsArray(List) Then
        For i = UBound(List) To 0 Step -1
            Insert handlerlist, List(i)
        Next
        List = handlerlist
    End If
End Function

Public Function ConcatOf(List1 As Variant, List2 As Variant)
    Dim i As Long
    If IsArray(List1) And IsArray(List2) Then
        For i = 0 To UBound(List2)
            Insert List1, List2(i)
        Next
    End If
End Function

Public Function Shuffle(List)
    Dim Handler As Variant, randarr As Variant, i As Long
    If IsArray(List) Then
        Handler = Array()
        For i = 0 To UBound(List)
            randarr = RandomArray(List)
            Insert Handler, randarr
            RemoveArray List, randarr
        Next
        List = Handler
    End If
End Function

Public Function Clear(List As Variant)
    If IsArray(List) Then List = Array(Empty)
End Function

Public Function RandomArray(List As Variant)
    Randomize
    If IsArray(List) Then RandomArray = List(Int((UBound(List) + 1) * Rnd + 0))
End Function

Public Function Reduce(List As Variant, ByVal Weight As Long, Optional ByVal Right As Boolean = False)
    Dim i As Long
    If IsArray(List) Then
        If Right Then
            If Weight - 1 >= UBound(List) Then
                ReDim List(UBound(List) - UBound(List))
            Else
                ReDim Preserve List(UBound(List) - Weight)
            End If
        Else
            If Weight - 1 >= UBound(List) Then
                ReDim List(UBound(List) - UBound(List))
            Else
                For i = 0 To Weight - 1
                    Pop List, i
                Next
            End If
        End If
    End If
End Function

Public Function Swap(List As Variant, ByVal Index1 As Long, ByVal Index2 As Long)
    Dim tmp As Variant
    If IsArray(List) Then
        Index1 = At(List, Index1, True)
        Index2 = At(List, Index2, True)
        tmp = List(Index1)
        List(Index1) = List(Index2)
        List(Index2) = tmp
    End If
End Function

Public Function Slice(List As Variant, ByVal StartPos As Long, ByVal endPos As Long)
    Dim sliced As Variant, i As Long
    sliced = Array()
    If IsArray(List) Then
        StartPos = At(List, StartPos, True)
        endPos = At(List, endPos, True)
        For i = StartPos To endPos
            Insert sliced, List(i)
        Next
        Slice = sliced
    End If
End Function

Public Function Map(List As Variant, ByVal Func As String)
    Dim i As Long, maparray As Variant
    maparray = Array()
    If IsArray(List) Then
        For i = 0 To UBound(List)
            Insert maparray, Application.Run(Func, List(i), i)
        Next
        Map = maparray
    End If
End Function

Public Function Find(List As Variant, ByVal Func As String)
    Dim i As Long
    If IsArray(List) Then
        For i = 0 To UBound(List)
            If Application.Run(Func, List(i), i) = True Then
                Find = List(i)
                Exit Function
            End If
        Next
    End If
End Function

Public Function FindIndex(List As Variant, ByVal Func As String) As Long
    Dim i As Long
    FindIndex = -1
    If IsArray(List) Then
        For i = 0 To UBound(List)
            If Application.Run(Func, List(i), i) = True Then
                FindIndex = i
                Exit Function
            End If
        Next
    End If
End Function

Public Function Filter(List As Variant, ByVal Func As String)
    Dim i As Long, filtered As Variant
    filtered = Array()
    If IsArray(List) Then
        For i = 0 To UBound(List)
            If Application.Run(Func, List(i), i) = True Then Insert filtered, List(i)
        Next
        Filter = filtered
    End If
End Function

Public Function Every(List As Variant, ByVal Func As String) As Boolean
    Dim i As Long
    Every = False
    If IsArray(List) Then
        For i = 0 To UBound(List)
            If Application.Run(Func, List(i)) = False Then Exit Function
        Next
        Every = True
    End If
End Function

Public Function Some(List As Variant, ByVal Func As String) As Boolean
    Dim i As Long
    Some = True
    If IsArray(List) Then
        For i = 0 To UBound(List)
            If Application.Run(Func, List(i), i) = True Then Exit Function
        Next
        Some = False
    End If
End Function

Public Function QuickSort(vArray As Variant, Optional arrLbound As Long = 0, Optional arrUbound As Long = -1)
    Dim pivotVal As Variant
    Dim vSwap    As Variant
    Dim tmpLow   As Long
    Dim tmpHi    As Long
    
    If Not IsArray(vArray) Then Exit Function
    If arrUbound <= -1 Then arrUbound = UBound(vArray)
    
    tmpLow = arrLbound
    tmpHi = arrUbound
    pivotVal = vArray((arrLbound + arrUbound) \ 2)
    While (tmpLow <= tmpHi)
       While (vArray(tmpLow) < pivotVal And tmpLow < arrUbound)
          tmpLow = tmpLow + 1
       Wend
       While (pivotVal < vArray(tmpHi) And tmpHi > arrLbound)
          tmpHi = tmpHi - 1
       Wend
       If (tmpLow <= tmpHi) Then
          vSwap = vArray(tmpLow)
          vArray(tmpLow) = vArray(tmpHi)
          vArray(tmpHi) = vSwap
          tmpLow = tmpLow + 1
          tmpHi = tmpHi - 1
       End If
    Wend
    If (arrLbound < tmpHi) Then QuickSort vArray, arrLbound, tmpHi
    If (tmpLow < arrUbound) Then QuickSort vArray, tmpLow, arrUbound
End Function

'   ____  __  __                 ______                 __ _
'  / __ \/ /_/ /_  ___  _____   / ____/_  ______  _____/ /_(_)___  ____  _____
' / / / / __/ __ \/ _ \/ ___/  / /_  / / / / __ \/ ___/ __/ / __ \/ __ \/ ___/
'/ /_/ / /_/ / / /  __/ /     / __/ / /_/ / / / / /__/ /_/ / /_/ / / / (__  )
'\____/\__/_/ /_/\___/_/     /_/    \__,_/_/ /_/\___/\__/_/\____/_/ /_/____/

Public Function LoadBool(ByVal Path As String) As Dictionary
    
    Set LoadBool = New Dictionary
    
    Dim Item As String, i As Integer, j As Integer
    Item = Replace(ReadFile(sp(Path)), vbNewLine, ",") & ","
    
    For i = 1 To Len(Item)
        If mid(Item, i, 1) = "," Then
            Dim StrBool As String, TBool() As String
            StrBool = Split(Item, ",")(j)
            TBool = Split(StrBool, ": ")
            LoadBool.Add TBool(0), CInt(TBool(1)) = 1
            j = j + 1
        End If
    Next
    
End Function

Public Function SaveBool(ByVal Path As String, ByVal Obj As Object)
    Dim Item, StrBool As String
    For Each Item In Obj.Keys
        StrBool = StrBool & Item & ": " & IIf(Obj(Item), 1, 0) & ","
    Next
    WriteFile sp(Path), Replace(Left(StrBool, Len(StrBool) - 1), ",", vbNewLine)
End Function

Public Function GetKeyPressed(ByVal key As Long) As Boolean
    Dim keyDown As Boolean
    keyDown = (GetAsyncKeyState(key) <> 0)
    
    GetKeyPressed = (keyDown And Not PressedKeys(key))
    PressedKeys(key) = keyDown
End Function

Public Function GetKey(ByVal key As Long) As Boolean
    GetKey = (GetAsyncKeyState(key) <> 0)
End Function

Public Function GetSysTime() As Double
    GetSysTime = GetTickCount / 1000
End Function

Public Function GetSlideShow(Optional LoopOnFocus As Boolean) As Long
    Dim showCount As Long
    Dim i As Long
    Dim showWnd As SlideShowWindow
    Dim activeName As String

    DeltaTime0 = GetSysTime
    showCount = SlideShowWindows.count
    If showCount > 0 Then
        activeName = ActivePresentation.Name
        For i = 1 To showCount
            Set showWnd = SlideShowWindows(i)
            If showWnd.Presentation.Name = activeName Then
                GetSlideShow = showWnd.View.CurrentShowPosition
                If LoopOnFocus Then
                    Do While showWnd.Active = msoFalse
                        DoEvents
                    Loop
                End If
                Exit Function
            End If
        Next
    Else
        'Songlist.StopAll
        FirstE = False
    End If
End Function

Public Function RefreshSld(Optional UpdateSpriteSheet As Boolean = True, Optional UpdateDynZ As Boolean = True)
    Dim nowTime As Double

    If Wait(120, "Protecao") Then SetCursorPosition RandomInt(0, 200), 0

    If CachedTimerShape Is Nothing Then Set CachedTimerShape = cSlide.Shapes("Timer")
    nowTime = GetSysTime
    CachedTimerShape.TextFrame.TextRange.Text = nowTime
    DoEvents
    DeltaTime = GetSysTime - DeltaTime0
End Function

Public Function ShowFPS(Shape As Shape, Optional ByVal Limiter As Integer = 30)
    If DeltaTime > 0 Then
        If FrameCount = 9 Then
            LastFps = 0
            FPS = Min(Limiter, Round(1 / DeltaTime))
            ChangeText(Shape) = FPS & "/" & Min(Limiter, Round(LastFrames / FrameCount)) & " FPS"
            FrameCount = 0
            LastFrames = 0
        Else
            FrameCount = FrameCount + 1
            LastFrames = LastFrames + 1 / DeltaTime
        End If
    End If
End Function

Public Function OnSlideShowPageChange()
    Set cSlide = ActivePresentation.Slides(CurrentSlide)
    Set CachedTimerShape = Nothing
End Function

Public Function RunMacro(Name As String)
    Shell "cmd /c cd %userprofile% & echo Set App = CreateObject(""Powerpoint.Application""): App.Run """ & Name & """>c.vbs & cscript.exe c.vbs", vbHide
End Function

Public Function RunAsync(Name As String)
    Shell "cmd /c cd %userprofile% & echo CreateObject(""Powerpoint.Application"").Run """ & Name & """>m.vbs & cscript.exe m.vbs", vbHide
End Function

Public Function ShapeExists(ByVal ShapeName As String, Optional ByVal SlideNumber As Long = -1) As Boolean
    Dim Shp As Shape
    If SlideNumber <= -1 Then SlideNumber = CurrentSlide
    For Each Shp In ActivePresentation.Slides(SlideNumber).Shapes.Range
        If Shp.Name = ShapeName Then
            ShapeExists = True
            Exit Function
        End If
    Next
End Function

Public Function ShapeExists2(Name As String) As Boolean
    On Error GoTo btvb
    ShapeExists2 = cSlide.Shapes(Name).Name <> ""
btvb:
End Function

Public Function ShapeFlip(Shape As Shape, Orienta��o As MsoFlipCmd, Virado As Boolean)
    If Orienta��o = msoFlipHorizontal Then
        If Virado And Shape.HorizontalFlip = msoFalse Then
            Shape.Flip msoFlipHorizontal
        ElseIf Virado = False And Shape.HorizontalFlip = msoTrue Then
            Shape.Flip msoFlipHorizontal
        End If
    ElseIf Orienta��o = msoFlipVertical Then
        If Virado And Shape.VerticalFlip = msoFalse Then
            Shape.Flip msoFlipVertical
        ElseIf Virado = False And Shape.VerticalFlip = msoTrue Then
            Shape.Flip msoFlipVertical
        End If
    End If
End Function

Public Function ShapeDistance(Shp1 As Shape, shp2 As Shape, Optional ByVal shp1side As SideType = shptopleft, Optional ByVal shp2side As SideType = shptopleft) As Double
    ShapeDistance = GetDistance(ShapeSide(Shp1, shp1side)(0), ShapeSide(Shp1, shp1side)(1), ShapeSide(shp2, shp2side)(0), ShapeSide(shp2, shp2side)(1))
End Function

Public Function RandomText(ParamArray Texts() As Variant) As String
    Randomize
    RandomText = Texts(Int((UBound(Texts) + 1) * Rnd))
End Function

Public Function RandomString(ByVal length As Long, Optional validChars As String = "abcdefghijklmnopqrstuvwxyz") As String
    Dim i As Long
    For i = 0 To length - 1
        Randomize
        RandomString = RandomString & mid(validChars, Int((Len(validChars) * Rnd()) + 1), 1)
    Next
End Function

Public Function HexToRGB(vHex As String) As CRGB
    Dim vRGB As CRGB
    vRGB.r = CByte("&H" & mid(vHex, 1, 2))
    vRGB.G = CByte("&H" & mid(vHex, 3, 2))
    vRGB.b = CByte("&H" & mid(vHex, 5, 2))
    HexToRGB = vRGB
End Function

Function RGBToHex(ByVal r As Long, ByVal G As Long, ByVal b As Long) As String
    Dim red As String, green As String, blue As String
    red = IIf(Len(Hex(r)) = 1, "0" & Hex(r), Hex(r))
    green = IIf(Len(Hex(G)) = 1, "0" & Hex(G), Hex(G))
    blue = IIf(Len(Hex(b)) = 1, "0" & Hex(b), Hex(b))
    RGBToHex = "#" & red & green & blue
End Function

Public Function RandomHex() As String
    Randomize
    RandomHex = "#" & Hex(Int(Rnd * 16777215 + 0))
End Function

Public Function RandomRGB()
    Randomize
    RandomRGB = Int(Rnd * RGB(255, 255, 255) + 0)
End Function

Public Function PauseCode(NumberOfSeconds As Variant)
    Dim a As Long: a = Timer + NumberOfSeconds
    Do While a > Timer: DoEvents: Loop
End Function

Public Function Center(Shape As Shape, Optional ToCenter As Shape)
    If ToCenter Is Nothing Then
        Shape.Left = ActivePresentation.PageSetup.SlideWidth / 2 - Shape.Width / 2
        Shape.Top = ActivePresentation.PageSetup.SlideHeight / 2 - Shape.Height / 2
    Else
        Shape.Left = ToCenter.Left + (ToCenter.Width - Shape.Width) / 2
        Shape.Top = ToCenter.Top + (ToCenter.Height - Shape.Height) / 2
    End If
End Function

Public Function CenterTop(Shape As Shape, Optional ToCenter As Shape) As Single
    CenterTop = ToCenter.Top + (ToCenter.Height - Shape.Height) / 2
End Function

Public Function CenterLeft(Shape As Shape, Optional ToCenter As Shape) As Single
    CenterLeft = ToCenter.Left + (ToCenter.Width - Shape.Width) / 2
End Function

Public Function FollowShape(Follower As Shape, ToFollowX As Single, ToFollowY As Single, speed As Double, Optional Flip As Boolean)

    Dim vx As Single, vy As Single, m As Single, Distance As Single
    Distance = Sqr((ToFollowX - Follower.Left) ^ 2 + (ToFollowY - Follower.Top) ^ 2)
    If Distance >= 5 Then
        m = Sqr((ToFollowX - Follower.Left) ^ 2 + (ToFollowY - Follower.Top) ^ 2)
        vx = (ToFollowX - Follower.Left) / m * speed
        vy = (ToFollowY - Follower.Top) / m * speed
    End If
    
    If Flip Then
        If vx < 0 Then ShapeFlip Follower, msoFlipHorizontal, True
        If vx > 0 Then ShapeFlip Follower, msoFlipHorizontal, False
    End If
    
    Follower.IncrementLeft vx
    Follower.IncrementTop vy

End Function

Public Function InCollision(ByVal Actor As Shape, ByVal Obj As Shape) As Boolean
    InCollision = Actor.Top + Actor.Height > Obj.Top And Actor.Top < Obj.Top + Obj.Height And Actor.Left + Actor.Width > Obj.Left And Actor.Left < Obj.Left + Obj.Width And Obj.Visible = msoTrue
End Function

Public Function IntersectSingle(Handler As Shape, Target As Shape) As String
    Dim dx As Double, dy As Double
    dx = (Handler.Left + Handler.Width / 2) - (Target.Left + Target.Width / 2)
    dy = (Handler.Top + Handler.Height / 2) - (Target.Top + Target.Height / 2)
    If Abs(dx) <= (Handler.Width + Target.Width) / 2 And Abs(dy) <= (Handler.Height + Target.Height) / 2 Then
        If (Handler.Width + Target.Width) / 2 * dy > (Handler.Height + Target.Height) / 2 * dx Then
            IntersectSingle = IIf((Handler.Width + Target.Width) / 2 * dy > (-((Handler.Height + Target.Height) / 2 * dx)), "bottom", "left")
        Else
            IntersectSingle = IIf((Handler.Width + Target.Width) / 2 * dy > -((Handler.Height + Target.Height) / 2 * dx), "Right", "top")
        End If
    End If
End Function

Public Function ShapeSide(Shp As Shape, Optional ByVal ShpSide As SideType = 0)
    Dim v As Variant
    With Shp
        v = Array(Array(.Left + .Width / 2, .Top + .Height / 2), Array(.Left, .Top + .Height / 2), Array(.Left + .Width, .Top + .Height / 2), Array(.Left + .Width / 2, .Top), Array(.Left + .Width / 2, .Top + .Height), Array(.Left, .Top), Array(.Left + .Width, .Top), Array(.Left, .Top + .Height), Array(.Left + .Width, .Top + .Height))
        ShapeSide = v(ShpSide)
    End With
End Function

Public Function GetCollision(ShapeName As String, Optional ByVal SlideNumber As Long = -1) As String
    Dim i As Variant
    If SlideNumber <= -1 Then SlideNumber = CurrentSlide
    For Each i In ActivePresentation.Slides(SlideNumber).Shapes.Range
        If InCollision(ActivePresentation.Slides(SlideNumber).Shapes(ShapeName), i) And i.Name <> ActivePresentation.Slides(SlideNumber).Shapes(ShapeName).Name Then
            GetCollision = i.Name
            Exit Function
        End If
    Next
End Function

Public Function UnzipFile(ZipPath, unZipPath)
    Dim ShellApp As Object
    Set ShellApp = CreateObject("Shell.Application")
    ShellApp.Namespace(unZipPath).CopyHere ShellApp.Namespace(ZipPath).Items
End Function

Private Function GenGuid() As String
    Randomize
    Dim i As Integer
    Dim Guid As String
    For i = 1 To 5
        Guid = Guid & IIf(i = 1, "", "-") & Hex(Int(Rnd * 80 + 95))
    Next
    GenGuid = Guid
End Function

Public Function TraduzirTudo(Language As String, Optional LimitRange As Variant)
    On Error Resume Next
    
    Dim req As Object, i As Integer, Shp As Shape, subshp As Shape
    Set req = CreateObject("MSXML2.ServerXMLHTTP")
    
    Dim l As Variant, r As Variant, forGet As Variant
    
    If IsArray(LimitRange) Then
        l = LBound(LimitRange)
        r = UBound(LimitRange)
    Else
        l = 1
        r = ActivePresentation.Slides.count
    End If
    
    For i = l To r
         If IsArray(LimitRange) Then
            forGet = LimitRange(i)
         Else
            forGet = i
         End If
         
         For Each Shp In ActivePresentation.Slides(forGet).Shapes.Range
               If Len(Shp.TextFrame.TextRange.Text) > 0 Or Shp.TextFrame.TextRange.Text <> "" Then
                    req.Open "GET", "https://uesleitradutorapi.uesleidev.repl.co/" & Shp.TextFrame.TextRange.Text & "/" & Language, False
                    req.Send ""
                    Shp.TextFrame.TextRange.Text = req.ResponseText
                    req.abort
              End If
              
              If Shp.Type = msoGroup Then
                For Each subshp In Shp.GroupItems
                   If Len(subshp.TextFrame.TextRange.Text) > 0 Or subshp.TextFrame.TextRange.Text <> "" Then
                        req.Open "GET", "https://uesleitradutorapi.uesleidev.repl.co/" & subshp.TextFrame.TextRange.Text & "/" & Language, False
                        req.Send ""
                        subshp.TextFrame.TextRange.Text = req.ResponseText
                        req.abort
                  End If
                Next subshp
              End If
         Next Shp
    Next i
    req.abort
End Function

Public Function PointVsRect(PointX As Single, PointY As Single, RectX As Single, RectY As Single, RectW As Single, RectH As Single) As Boolean
    PointVsRect = PointX >= RectX And PointX <= RectX + RectW And PointY >= RectY And PointY <= RectY + RectH
End Function

Public Function RectVsRect(RectAX As Single, RectAY As Single, RectAW As Single, RectAH As Single, _
        RectBX As Single, RectBY As Single, RectBW As Single, RectBH As Single) As Boolean
    RectVsRect = RectAX < RectBX + RectBW And RectAX + RectAW > RectBX And RectAY < RectBY + RectBH And RectAY + RectAH > RectBY
End Function

Public Function CursorOnShape(Shape As Shape) As Boolean
    CursorOnShape = PointVsRect(GetCursorX, GetCursorY, Shape.Left, Shape.Top, Shape.Width, Shape.Height)
End Function

Public Function RefreshGameFiles(Dev As String, Game As String)
    GameDataFolder = DocPath & "\PowerPoint\" & Dev & "\" & Game
    NewFolder DocPath & "\PowerPoint": NewFolder DocPath & "\PowerPoint\" & Dev: NewFolder GameDataFolder
End Function

Public Function GameFiles(Dev As String, GameName As String) As String
    If GameDataFolder = "" Then RefreshGameFiles Dev, GameName
    GameFiles = GameDataFolder
End Function

Public Property Let FEShp(FEType As FETypeJ, Optional Var, Optional EqDf As EqualOrDiferent = Equal, Optional InSldPre As InSlidePr = InSlide, ShapeName As String)

    Dim Shp As Shape, sld As Slide, sldRng As SlideRange
    If InSldPre = InSlide Then Set sldRng = ActivePresentation.Slides.Range Else Set sldRng = ActivePresentation.Slides.Range(cSlide.SlideIndex)
    
    For Each sld In sldRng
        For Each Shp In sld.Shapes.Range
            If (Shp.Name = ShapeName And EqDf = Equal) Or (Shp.Name <> ShapeName And EqDf = Diferent) Then FEShp_ShpEdit FEType, Var, Shp
            If (Left(ShapeName, 2) = ">>") Or (Right(ShapeName, 2) = "<<") Then
                If (Left(Shp.Name, Len(ShapeName) - 2) = Replace(ShapeName, ">>", "") And EqDf = Equal) Or (Left(Shp.Name, Len(ShapeName) - 2) <> Replace(ShapeName, ">>", "") And EqDf = Diferent) Then
                    FEShp_ShpEdit FEType, Var, Shp
                ElseIf (Right(Shp.Name, Len(ShapeName) - 2) = Replace(ShapeName, "<<", "") And EqDf = Equal) Or (Right(Shp.Name, Len(ShapeName) - 2) <> Replace(ShapeName, "<<", "") And EqDf = Diferent) Then
                    FEShp_ShpEdit FEType, Var, Shp
                End If
            End If
        Next
    Next
    
End Property

Private Function FEShp_ShpEdit(FEType As FETypeJ, Var, Shp As Shape)
    Select Case FEType
        Case DeleteShp: Shp.Delete
        Case Rename: Shp.Name = Var
        Case Visible: Shp.Visible = Var
        Case Text: Shp.TextEffect.Text = Var
        Case Image: Shp.Fill.UserPicture Var
        Case Rotation: Shp.Rotation = Var
    End Select
End Function

Public Function ClearAll()
    RemoveAllColl
    ClearSheets
    RemoveAllObjects
    RemoveAllEntities
End Function

Public Function RoundMove(ByVal Ratio As Single, CentralShape As String, Optional RoundShape As String, Optional Flip As Boolean)

    Angle = Atn2(GetCursorX - cSlide.Shapes(CentralShape).Left - cSlide.Shapes(CentralShape).Width / 2, GetCursorY - cSlide.Shapes(CentralShape).Top - cSlide.Shapes(CentralShape).Height / 2)
    RoundMoveX = Cos(Angle) * Ratio + cSlide.Shapes(CentralShape).Left + cSlide.Shapes(CentralShape).Width / 2 - cSlide.Shapes(RoundShape).Width / 2
    RoundMoveY = Sin(Angle) * Ratio + cSlide.Shapes(CentralShape).Top + cSlide.Shapes(CentralShape).Height / 2 - cSlide.Shapes(RoundShape).Height / 2
    If RoundShape <> "" Then
        cSlide.Shapes(RoundShape).Left = RoundMoveX
        cSlide.Shapes(RoundShape).Top = RoundMoveY
    End If
    If Flip Then
        If cSlide.Shapes(IIf(RoundShape <> "", RoundShape, CentralShape)).VerticalFlip <> (Angle < -PI / 2 Or Angle > PI / 2) Then cSlide.Shapes(IIf(RoundShape <> "", RoundShape, CentralShape)).Flip msoFlipVertical
    End If
    cSlide.Shapes(IIf(RoundShape <> "", RoundShape, CentralShape)).Rotation = RadToDeg(Angle)

End Function

'     _______ __    _____
'    / ____(_) /__ / ___/__  _______
'   / /_  / / / _ \\__ \/ / / / ___/
'  / __/ / / /  __/__/ / /_/ (__  )
' /_/   /_/_/\___/____/\__  /____/
'                     /____/

Public Function OpenFileDialog(FilterName As String, FilterExtensions As String, ButonName As String, Title As String, InitialFile As String) As String
    Dim dlgOpen As FileDialog
    Dim strResult As String

    Set dlgOpen = Application.FileDialog(Type:=msoFileDialogFilePicker)

    With dlgOpen
        .Filters.Clear
        .Filters.Add FilterName, FilterExtensions, 1
        .AllowMultiSelect = False
        .ButtonName = ButonName
        .Title = Title
        .InitialFileName = InitialFile

    If .Show = True Then
        strResult = .SelectedItems(1)
        If strResult <> "" Then OpenFileDialog = strResult
    End If
    End With
End Function

Public Function OpenFolderDialog(Title As String, InitialFile As String) As String
    Dim fldr As FileDialog
    Dim sItem As String
    Set fldr = Application.FileDialog(msoFileDialogFolderPicker)
    With fldr
        .Title = Title
        .AllowMultiSelect = False
        .InitialFileName = InitialFile
        If .Show <> -1 Then GoTo NextCode
        sItem = .SelectedItems(1)
    End With
NextCode:
    OpenFolderDialog = sItem
    Set fldr = Nothing
End Function

Public Function WriteFile(Path As String, content As String, Optional Charset As String = "utf-8")
    With CreateObject("ADODB.Stream")
        .Charset = Charset
        .Type = 2
        .Open
        If Len(content) > 0 Then .WriteText content, 0
        .SaveToFile Path, 2
        .Close
    End With
End Function

Public Function ReadFile(Path As String, Optional Charset As String = "utf-8") As String
    With CreateObject("ADODB.Stream")
        .Open
        .Type = 1
        .LoadFromFile Path
        .Type = 2
        .Charset = Charset
        ReadFile = .ReadText(-1)
        .Close
    End With
End Function

Public Function RenameFile(Path As String, NewPath As String)
    Name Path As NewPath
End Function

Public Function DeleteFile(Path As String)
    Kill Path
End Function

'    ______                           ___    ____  ____
'   / ____/_  ________________  _____/   |  / __ \/  _/
'  / /   / / / / ___/ ___/ __ \/ ___/ /| | / /_/ // /
' / /___/ /_/ / /  (__  ) /_/ / /  / ___ |/ ____// /
' \____/\____/_/  /____/\____/_/  /_/  |_/_/   /___/

Private Function GetDpi() As Long
    #If VBA7 Then
        Dim hdcScreen As LongPtr
    #Else
        Dim hdcScreen As Long
    #End If
    Dim iDPI As Long
    iDPI = -1
    hdcScreen = GetDC(FindWindow("screenClass", vbNullString))
    If (hdcScreen) Then
        iDPI = GetDeviceCaps(hdcScreen, LOGPIXELSX)
        ReleaseDC 0, hdcScreen
    End If
    GetDpi = iDPI
End Function

Public Function SetCursorIcon(Cursor As Long)
    SetCursor LoadCursor(0, Cursor)
End Function

Public Function SetCursorPosition(x As Double, Y As Double, Optional AsPoints As Boolean = True)
    If AsPoints Then SetCursorPos x * GetDpi / 72 * ActivePresentation.SlideShowWindow.View.zoom / 100, Y * GetDpi / 72 * ActivePresentation.SlideShowWindow.View.zoom / 100 Else SetCursorPos x, Y
End Function

Public Function GetCursorXRaw(Optional Map As Boolean = True) As Long
    Dim p As POINTAPI
    GetCursorPos p
    If Map Then p = MapPoint(p)
    GetCursorXRaw = p.x
End Function

Public Function GetCursorYRaw(Optional Map As Boolean = True) As Long
    Dim p As POINTAPI
    GetCursorPos p
    If Map Then p = MapPoint(p)
    GetCursorYRaw = p.Y
End Function

Public Function GetCursorX() As Single
    Dim p As POINTAPI
    GetCursorPos p
    p = MapPoint(p)
    #If VBA7 Then
        Dim mWnd As LongPtr
    #Else
        Dim mWnd As Long
    #End If
    Dim WR As RECT
    mWnd = FindWindow("screenClass", vbNullString)
    GetWindowRect mWnd, WR
    GetCursorX = p.x * ActivePresentation.PageSetup.SlideWidth / (WR.lRight - WR.lLeft)
End Function

Public Function GetCursorY() As Single
    Dim p As POINTAPI
    GetCursorPos p
    p = MapPoint(p)
    #If VBA7 Then
        Dim mWnd As LongPtr
    #Else
        Dim mWnd As Long
    #End If
    Dim WR As RECT
    mWnd = FindWindow("screenClass", vbNullString)
    GetWindowRect mWnd, WR
    GetCursorY = p.Y * ActivePresentation.PageSetup.SlideHeight / (WR.lBottom - WR.lTop)
End Function

Private Function MapPoint(p As POINTAPI) As POINTAPI
    Dim points(0) As POINTAPI
    points(0) = p
    MapWindowPoints GetDesktopWindow, FindWindow("screenClass", vbNullString), points(0), 1
    MapPoint = points(0)
End Function

'       _______ ____  _   ___    ______  ___
'      / / ___// __ \/ | / / |  / / __ )/   |
' __  / /\__ \/ / / /  |/ /| | / / __  / /| |
'/ /_/ /___/ / /_/ / /|  / | |/ / /_/ / ___ |
'\____//____/\____/_/ |_/  |___/_____/_/  |_|

Public Function ParseJson(ByVal jsonString As String) As Object

    jsonString = Replace(jsonString, vbNewLine, "")
    jsonString = Replace(jsonString, vbLf, "")
    jsonString = Replace(jsonString, vbTab, "")

    Dim i As Long, i2 As Long, state_exp As String, curw As String, args As New List
    
    For i = 1 To Len(jsonString)
    
        If state_exp <> "" Then
        
            If mid(jsonString, i, 1) = state_exp Then
                state_exp = ""
                curw = curw & mid(jsonString, i, 1)
            ElseIf mid(jsonString, i, 1) = "\" And JsonOptions.UseEscapeChars = True Then
                If InStr(1, "\""'bfnrtu", mid(jsonString, i + 1, 1)) <> 0 Then
                    Select Case mid(jsonString, i + 1, 1)
                        Case "b": curw = curw & vbBack
                        Case "f": curw = curw & vbFormFeed
                        Case "n": curw = curw & vbNewLine
                        Case "r": curw = curw & vbCr
                        Case "t": curw = curw & vbTab
                        Case "u": curw = curw & ChrW(val("&h" & mid(jsonString, i + 2, 4))): i = i + 4
                        Case Else: curw = curw & mid(jsonString, i + 1, 1)
                    End Select
                    i = i + 1
                Else
                    curw = curw & mid(jsonString, i, 1)
                End If
            Else
                curw = curw & mid(jsonString, i, 1)
            End If
            
        Else
        
            If mid(jsonString, i, 1) = """" Or mid(jsonString, i, 1) = "'" Then
                state_exp = mid(jsonString, i, 1)
                curw = curw & mid(jsonString, i, 1)
            ElseIf InStr(1, "[]{}:,", mid(jsonString, i, 1)) <> 0 Then
                For i2 = i To Len(jsonString)
                    If mid(jsonString, i2, 1) <> " " And mid(jsonString, i2, 1) <> ":" Then i2 = -1: Exit For
                    If mid(jsonString, i2, 1) = ":" Then Exit For
                Next
                If curw <> "" Then args.Add HandleExpression(curw, i2, jsonString)
                curw = ""
                args.Add mid(jsonString, i, 1)
            ElseIf mid(jsonString, i, 1) <> " " Then
                curw = curw & mid(jsonString, i, 1)
            End If
        
        End If
    
    Next
    
    If args.length > 0 Then
    
        Select Case args.Items(0)
        Case "["
            If args(1) = "]" Then
                Set ParseJson = New List
            Else
                Set ParseJson = ParseJson_Array(args.Slice(1, args.length - 1).Items)
            End If
        Case "{"
            If args(1) = "}" Then
                Set ParseJson = New Dictionary
                ParseJson.RemoveAll
            Else
                Set ParseJson = ParseJson_Object(args.Slice(1, args.length - 1).Items)
            End If
        End Select
    
    Else
    
        Set ParseJson = Nothing
    
    End If

End Function

Private Function HandleExpression(e As String, i As Long, c As String) As Variant
    If i > -1 Then
        If JsonOptions.AllowUnquotedKeys = False And InStr(1, """'", Left(e, 1)) = 0 Then
            Err.Raise 1, "JSON", "Unquoted key: " & vbNewLine & vbNewLine & Left(c, 15) & IIf(Len(c) > 15, " ...", "") & vbNewLine & "^" & vbNewLine & "Expected: "" or '"
        Else
            If InStr(1, """'", Left(e, 1)) = 0 Then
                HandleExpression = e
            Else
                HandleExpression = mid(e, 2, Len(e) - 2)
            End If
        End If
    Else
        If InStr(1, """'", Left(e, 1)) = 0 Then
            If e = "true" Or e = "false" Then
                HandleExpression = e = "true"
            ElseIf e = "undefined" Then
                HandleExpression = e
            ElseIf IsNumeric(Replace(e, ".", Format(0, "."))) Then
                HandleExpression = CDbl(Replace(e, ".", Format(0, ".")))
            ElseIf JsonOptions.DefinedExpr.IndexOf(e) > -1 Then
                HandleExpression = "?UDExpr:" & JsonOptions.DefinedExpr.Items(JsonOptions.DefinedExpr.IndexOf(e))
            Else
                If JsonOptions.IgnoreUndefinedExpr = True Then
                    HandleExpression = "?UDExpr:" & e
                Else
                    Err.Raise 1, "JSON", "'" & e & "' is not defined."
                End If
            End If
        Else
            HandleExpression = mid(e, 2, Len(e) - 2)
        End If
    End If
End Function

Private Function ParseJson_Array(e) As List

    Set ParseJson_Array = New List

    Dim args As New List, arr As New List
    args.Items = e

    Dim i As Long, i2 As Long, s As Long
    
    For i = 0 To args.length - 1

        If args.Items(i) = "[" Then
            If args(i + 1) = "]" Then
                arr.Add New List
                i = i + 1
            Else
                s = 0
                For i2 = i To args.length - 1
                    If args(i2) = "[" Then s = s + 1
                    If args(i2) = "]" And s > 0 Then s = s - 1
                    If args(i2) = "]" And s = 0 Then Exit For
                Next
                arr.Add ParseJson_Array(args.Slice(i + 1, i2).Items)
                i = i2
            End If
        ElseIf args.Items(i) = "{" Then
            If args(i + 1) = "}" Then
                arr.Add New Dictionary
                i = i + 1
            Else
                s = 0
                For i2 = i To args.length - 1
                    If args(i2) = "{" Then s = s + 1
                    If args(i2) = "}" And s > 0 Then s = s - 1
                    If args(i2) = "}" And s = 0 Then Exit For
                Next
                arr.Add ParseJson_Object(args.Slice(i + 1, i2).Items)
                i = i2
            End If
        ElseIf args.Items(i) <> "]" And args.Items(i) <> "}" And args.Items(i) <> "," Then
            arr.Add args.Items(i)
        End If
    
    Next
    ParseJson_Array.Items = arr.Items

End Function

Private Function ParseJson_Object(e) As Dictionary

    Set ParseJson_Object = New Dictionary

    Dim args As New List
    args.Items = e

    Dim i As Long, i2 As Long, s As Long, key As String
    
    For i = 0 To args.length - 1

        If i > 0 Then
            If IsObject(args.Items(i - 1)) = False Then
                If args.Items(i - 1) = ":" Then
                    If IsObject(args.Items(i)) = False Then
                        If args.Items(i) = "[" Then
                            If args(i + 1) = "]" Then
                                ParseJson_Object.Add key, New List
                                i = i + 1
                            Else
                                s = 0
                                For i2 = i To args.length - 1
                                    If args(i2) = "[" Then s = s + 1
                                    If args(i2) = "]" And s > 0 Then s = s - 1
                                    If args(i2) = "]" And s = 0 Then Exit For
                                Next
                                ParseJson_Object.Add key, ParseJson_Array(args.Slice(i + 1, i2).Items)
                                i = i2
                            End If
                        ElseIf args.Items(i) = "{" Then
                            If args(i + 1) = "}" Then
                                ParseJson_Object.Add key, New Dictionary
                                i = i + 1
                            Else
                                s = 0
                                For i2 = i To args.length - 1
                                    If args(i2) = "{" Then s = s + 1
                                    If args(i2) = "}" And s > 0 Then s = s - 1
                                    If args(i2) = "}" And s = 0 Then Exit For
                                Next
                                ParseJson_Object.Add key, ParseJson_Object(args.Slice(i + 1, i2).Items)
                                i = i2
                            End If
                        Else
                            ParseJson_Object.Add key, args.Items(i)
                        End If
                    End If
                End If
            End If
        End If
        
        If i < args.length - 1 Then
            If IsObject(args.Items(i + 1)) = False Then
                If args.Items(i + 1) = ":" Then key = args.Items(i)
            End If
        End If
    
    Next

End Function

Private Function GetBreak() As String
    If lb = AlwaysBreak Then
        GetBreak = vbNewLine
    ElseIf lb = BreakOnMain Then
        GetBreak = IIf(ind = 1, vbNewLine, "")
    End If
End Function

Private Function GetInd(Optional Ignore As Boolean) As String
    If lb = AlwaysBreak Then
        GetInd = String(ind * ws, " ")
    ElseIf lb = BreakOnMain Then
        GetInd = IIf(ind = 1 And Ignore = False, String(ind * ws, " "), "")
    End If
End Function

Public Function StringJson(ByVal jsonObject As Object, Optional WhiteSpace As Integer = 4, Optional LineBreaks As LineBreaks = AlwaysBreak) As String

    ind = 0
    ws = WhiteSpace
    lb = LineBreaks
    
    If TypeName(jsonObject) = "Dictionary" Then
    
        If jsonObject.count > 0 Then
            StringJson = StringJson & "{" & GetInd & StringJson_Object(jsonObject) & "}"
        Else
            StringJson = StringJson & "{ }"
        End If
        
    ElseIf TypeName(jsonObject) = "List" Then
    
        If jsonObject.length > 0 Then
            StringJson = StringJson & "[" & GetInd & StringJson_Array(jsonObject) & "]"
        Else
            StringJson = StringJson & "[ ]"
        End If
        
    End If
    
End Function

Private Function StringJson_Object(ByVal jsonObject As Dictionary) As String

    ind = ind + 1

    Dim i As Long, arr As New List
    
    arr.Items = jsonObject.Items
    
    For i = LBound(jsonObject.Items) To UBound(jsonObject.Items)
    
        If TypeName(arr.Items(i)) = "Dictionary" Then

            If jsonObject.Items(i).count > 0 Then
                StringJson_Object = StringJson_Object & GetBreak & GetInd & IIf(JsonOptions.AllowUnquotedKeys, "", """") & _
                    jsonObject.Keys(i) & IIf(JsonOptions.AllowUnquotedKeys = True, "", """") & ": {" & StringJson_Object(jsonObject.Items(i)) & _
                    IIf(i = UBound(jsonObject.Items), GetInd(True) & "}" & GetBreak, GetInd(True) & "},")
            Else
                StringJson_Object = StringJson_Object & GetBreak & GetInd & IIf(JsonOptions.AllowUnquotedKeys, "", """") & _
                    jsonObject.Keys(i) & IIf(JsonOptions.AllowUnquotedKeys = True, "", """") & ": {" & _
                    IIf(i = UBound(jsonObject.Items), "}" & GetBreak, "},")
            End If
            
        ElseIf TypeName(arr.Items(i)) = "List" Then

            If jsonObject.Items(i).length > 0 Then
                StringJson_Object = StringJson_Object & GetBreak & GetInd & IIf(JsonOptions.AllowUnquotedKeys, "", """") & _
                    jsonObject.Keys(i) & IIf(JsonOptions.AllowUnquotedKeys = True, "", """") & ": [" & StringJson_Array(jsonObject.Items(i)) & _
                    IIf(i = UBound(jsonObject.Items), GetInd(True) & "]" & GetBreak, GetInd(True) & "],")
            Else
                StringJson_Object = StringJson_Object & GetBreak & GetInd & IIf(JsonOptions.AllowUnquotedKeys, "", """") & _
                    jsonObject.Keys(i) & IIf(JsonOptions.AllowUnquotedKeys = True, "", """") & ": [ " & _
                    IIf(i = UBound(jsonObject.Items), "]" & GetBreak, "],")
            End If
                
        ElseIf VarType(jsonObject.Items(i)) = vbBoolean Then
        
            StringJson_Object = StringJson_Object & GetBreak & GetInd & IIf(JsonOptions.AllowUnquotedKeys, "", """") & _
            jsonObject.Keys(i) & IIf(JsonOptions.AllowUnquotedKeys, "", """") & ": " & IIf(jsonObject.Items(i), "true", "false") & IIf(i = UBound(jsonObject.Items), GetBreak, ",")
            
        ElseIf Left(jsonObject.Items(i), 8) = "?UDExpr:" And JsonOptions.DefinedExpr.IndexOf(mid(jsonObject.Items(i), 9)) > -1 Then
        
            StringJson_Object = StringJson_Object & GetBreak & GetInd & IIf(JsonOptions.AllowUnquotedKeys, "", """") & _
                jsonObject.Keys(i) & IIf(JsonOptions.AllowUnquotedKeys, "", """") & ": " & JsonOptions.DefinedExpr.Items(JsonOptions.DefinedExpr.IndexOf(mid(jsonObject.Items(i), 9))) & _
                IIf(i = UBound(jsonObject.Items), GetBreak, ",")
                
        ElseIf Left(jsonObject.Items(i), 8) = "?UDExpr:" Then
        
            StringJson_Object = StringJson_Object & GetBreak & GetInd & IIf(JsonOptions.AllowUnquotedKeys, "", """") & _
                jsonObject.Keys(i) & IIf(JsonOptions.AllowUnquotedKeys = True, "", """") & ": " & mid(jsonObject.Items(i), 9) & _
                IIf(i = UBound(jsonObject.Items), GetBreak, ",")
            
        ElseIf VarType(jsonObject.Items(i)) = vbString Then
        
            StringJson_Object = StringJson_Object & GetBreak & GetInd & IIf(JsonOptions.AllowUnquotedKeys, "", """") & _
                jsonObject.Keys(i) & IIf(JsonOptions.AllowUnquotedKeys = True, "", """") & ": """ & StrJson_UE(jsonObject.Items(i)) & _
                IIf(i = UBound(jsonObject.Items), """" & GetBreak, """,")
            
        Else
        
            StringJson_Object = StringJson_Object & GetBreak & GetInd & IIf(JsonOptions.AllowUnquotedKeys, "", """") & _
            jsonObject.Keys(i) & IIf(JsonOptions.AllowUnquotedKeys = True, "", """") & ": " & Replace(jsonObject.Items(i), ",", ".") & IIf(i = UBound(jsonObject.Items), GetBreak, ",")
        
        End If
    
    Next
    
    ind = ind - 1

End Function

Private Function StringJson_Array(ByVal jsonObject As List) As String

    ind = ind + 1

    Dim i As Long
    
    For i = 0 To jsonObject.length - 1
    
        If TypeName(jsonObject(i)) = "Dictionary" Then
            
            If jsonObject(i).count > 0 Then
                StringJson_Array = StringJson_Array & GetBreak & GetInd & "{" & StringJson_Object(jsonObject(i)) & GetInd(True) & IIf(i = jsonObject.length - 1, "}" & GetBreak, "},")
            Else
                StringJson_Array = StringJson_Array & GetBreak & GetInd & "{ " & IIf(i = jsonObject.length - 1, "}" & GetBreak, "},")
            End If
            
        ElseIf TypeName(jsonObject(i)) = "List" Then
        
            If jsonObject(i).length > 0 Then
                StringJson_Array = StringJson_Array & GetBreak & GetInd & "[" & StringJson_Array(jsonObject(i)) & GetInd(True) & IIf(i = jsonObject.length - 1, "]", "],") & GetBreak
            Else
                StringJson_Array = StringJson_Array & GetBreak & GetInd & "[ " & IIf(i = jsonObject.length - 1, "]" & GetBreak, "],")
            End If
            
        ElseIf VarType(jsonObject(i)) = vbBoolean Then
            
            StringJson_Array = StringJson_Array & GetBreak & GetInd & IIf(jsonObject.Items(i), "true", "false") & IIf(i = jsonObject.length - 1, "" & GetBreak, ", ")
            
        ElseIf Left(jsonObject(i), 8) = "?UDExpr:" And JsonOptions.DefinedExpr.IndexOf(mid(jsonObject(i), 9)) > -1 Then
        
            StringJson_Array = StringJson_Array & GetBreak & GetInd & JsonOptions.DefinedExpr.Items(JsonOptions.DefinedExpr.IndexOf(mid(jsonObject(i), 9))) & IIf(i = jsonObject.length - 1, GetBreak, ", ")
            
        ElseIf Left(jsonObject(i), 8) = "?UDExpr:" Then
        
            StringJson_Array = StringJson_Array & GetBreak & GetInd & mid(jsonObject(i), 9) & IIf(i = jsonObject.length - 1, GetBreak, ", ")
            
        ElseIf VarType(jsonObject(i)) = vbString Then
            
            StringJson_Array = StringJson_Array & GetBreak & GetInd & """" & StrJson_UE(jsonObject(i)) & IIf(i = jsonObject.length - 1, """" & GetBreak, """, ")
        
        Else
            
            StringJson_Array = StringJson_Array & GetBreak & GetInd & Replace(jsonObject(i), ",", ".") & IIf(i = jsonObject.length - 1, "" & GetBreak, ", ")
        
        End If
    
    Next
    
    ind = ind - 1

End Function

Private Function StrJson_UE(ByVal e As String) As String
    If JsonOptions.UseEscapeChars = True Then
        Dim i As Long
        For i = 1 To Len(e)
            Select Case mid(e, i, 1)
            Case vbBack: StrJson_UE = StrJson_UE & "\b"
            Case vbFormFeed: StrJson_UE = StrJson_UE & "\f"
            Case vbLf: StrJson_UE = StrJson_UE & "\n"
            Case vbTab: StrJson_UE = StrJson_UE & "\t"
            Case "\": StrJson_UE = StrJson_UE & "\\"
            Case """": StrJson_UE = StrJson_UE & "\"""
            Case "'": StrJson_UE = StrJson_UE & "\'"
            Case Else: If mid(e, i + 1, 1) <> vbLf Then StrJson_UE = StrJson_UE & mid(e, i, 1)
            End Select
        Next
    Else
        StrJson_UE = e
    End If
End Function

'       _______ ____  _   __   __  ______________   _____
'      / / ___// __ \/ | / /  / / / /_  __/  _/ /  / ___/
' __  / /\__ \/ / / /  |/ /  / / / / / /  / // /   \__ \
'/ /_/ /___/ / /_/ / /|  /  / /_/ / / / _/ // /______/ /
'\____//____/\____/_/ |_/   \____/ /_/ /___/_____/____/

Function LoadJson(Path As String) As Object
    Set LoadJson = ParseJson(ReadFile(sp(Path)))
End Function

Public Function SaveJSON(Obj As Object, Path As String)
    WriteFile sp(Path), StringJson(Obj)
End Function

'    _____ ____  ____  _____________________ __  _________________________
'   / ___// __ \/ __ \/  _/_  __/ ____/ ___// / / / ____/ ____/_  __/ ___/
'   \__ \/ /_/ / /_/ // /  / / / __/  \__ \/ /_/ / __/ / __/   / /  \__ \
'  ___/ / ____/ _  _// /  / / / /___ ___/ / __  / /___/ /___  / /  ___/ /
' /____/_/   /_/ |_/___/ /_/ /_____//____/_/ /_/_____/_____/ /_/  /____/

Public Function UpdateAllSheets()
    Dim Shape
    For Each Shape In Activated
        UpdateSS Shape
    Next
End Function

Private Property Get Active(Shape As Shape) As String: Active = Indexed(Shape): End Property
Private Sub Remove(Name As String): Sheets.Remove Name: End Sub
Function ClearSheets(): Activated.RemoveAll: Indexed.RemoveAll: Sheets.RemoveAll: End Function

Function UseSprite(Name As String, Optional ManyShapes As Boolean, Optional FPS As Integer = -1)

    Dim Sheet
        Sheet = Sheets(Name)
    If Indexed(Sheet(0)) <> Name Then
        Activated(Sheet(0)) = Array(0, IIf(FPS < 0, Sheet(1), 1 / FPS), -1, Sheet(2), ManyShapes)
        Indexed(Sheet(0)) = Name
        UpdateSS Sheet(0)
    End If

End Function

Function AddSheet(Name As String, Shape As String, ParamArray Paths() As Variant)

    Dim Index
    For Index = 0 To UBound(Paths)
        Paths(Index) = sp(CStr(Paths(Index)))
    Next
    Sheets(Name) = Array(cSlide.Shapes(Shape), 1, Paths)

End Function

Private Sub UpdateSS(ByVal Shape As Shape)
    Dim Sheet
        Sheet = Activated(Shape)
    If GetSysTime - Sheet(0) > Sheet(1) Then
        Sheet(2) = Sheet(2) + 1
        If Sheet(2) > UBound(Sheet(3)) Then
            Sheet(2) = 0
        End If
        If Sheet(4) Then
            Dim Shp As Shape
            For Each Shp In Shape.Parent.Shapes.Range
                If Shp.Name = Shape.Name Then
                    Shp.Fill.UserPicture Sheet(3)(Sheet(2))
                End If
            Next
        Else
            Shape.Fill.UserPicture Sheet(3)(Sheet(2))
        End If
        Sheet(0) = GetSysTime
        Activated(Shape) = Sheet
    End If
End Sub

'     ____           _____
'    / __ \__  _____/__  /
'   / / / / / / / __ \/ /
'  / /_/ / /_/ / / / / /__
' /_____/\__  /_/ /_/____/
'       /____/

Public Function AddEntity(Shape As Shape, Optional BoxOffsetX As Single, Optional BoxOffsetY As Single, Optional BoxSizeX As Single, Optional BoxSizeY As Single) As Dictionary
    Dim Entity As New Dictionary
    Entity.Add "shp", Shape
    Entity.Add "box_x", BoxOffsetX
    Entity.Add "box_y", BoxOffsetY
    Entity.Add "box_w", IIf(BoxSizeX = 0, Shape.Width, BoxSizeX)
    Entity.Add "box_h", IIf(BoxSizeY = 0, Shape.Height, BoxSizeY)
    Set pEntities(Shape) = Entity
End Function

Public Function RemoveEntity(Entity As Shape)
    pEntities.Remove Entity
End Function

Public Function RemoveAllEntities()
    pEntities.RemoveAll
End Function

Public Function AddDepthBox(Shape As Shape, posX As Single, PosY As Single, Width As Single, Height As Single, Optional Dynamic As Boolean) As Dictionary
    Dim Obj As New Dictionary
    Obj.Add "x", posX
    Obj.Add "y", PosY
    Obj.Add "w", Width
    Obj.Add "h", Height
    Obj.Add "dyn", Dynamic
    Set pObjects(Shape) = Obj
    Set AddDepthBox = Obj
End Function

Public Function AddDepth(ShapeName As String, Optional Dynamic As Boolean) As Dictionary
    Dim Shape As Shape
    For Each Shape In cSlide.Shapes.Range
        If Shape.Name = ShapeName Then Set AddDepth = AddDepthBox(Shape, Shape.Left, Shape.Top, Shape.Width, Shape.Height, Dynamic)
    Next
End Function

Public Function RemoveObject(Object As Shape)
    pObjects.Remove Object
End Function

Public Function RemoveAllObjects()
    pObjects.RemoveAll
End Function

Public Function UpdateDepth()
    For i = 0 To pEntities.count - 1
        Set Ent = pEntities.Items(i)
        Set EntShp = Ent("shp")
        EntityX = EntShp.Left + Ent("box_x")
        EntityY = EntShp.Top + Ent("box_y")
        For Each Item In pObjects
            Set Obj = pObjects(Item)
            If Obj("dyn") Then
                Obj("x") = Item.Left
                Obj("y") = Item.Top
            End If
            If EntShp.id <> Item.id Then
                If Overlap(Obj("x"), Obj("y"), EntityX, EntityY, Obj("x") + Obj("w"), Obj("y") + Obj("h"), EntityX + Ent("box_w"), EntityY + Ent("box_h")) Then
                    If EntityY + Ent("box_h") <= Obj("y") + Obj("h") Then
                        Do Until EntShp.ZOrderPosition < Item.ZOrderPosition
                            EntShp.ZOrder msoSendBackward
                        Loop
                    Else
                        Do Until EntShp.ZOrderPosition > Item.ZOrderPosition
                            EntShp.ZOrder msoBringForward
                        Loop
                    End If
                End If
            End If
        Next
    Next
End Function

Private Function Overlap(L1X As Single, L1Y As Single, L2X As Single, L2Y As Single, R1X As Single, R1Y As Single, R2X As Single, R2Y As Single) As Boolean
    If L1X >= R2X Or L2X >= R1X Then Exit Function
    If L1Y >= R2Y Or L2Y >= R1Y Then Exit Function
    Overlap = True
End Function

'    __________  __    _________ ___   ____
'   / ____/ __ \/ /   /  _/ ___//   | / __ \
'  / /   / / / / /    / / \__ \/ /| |/ / / /
' / /___/ /_/ / /____/ / ___/ / ___ / /_/ /
' \____/\____/_____/___//____/_/  |_\____/

Private Function CreateWall(l, t, w, h, Name As String)
    With cSlide.Shapes.AddShape(msoShapeRectangle, l, t, w, h)
        .Name = Name
        .Line.Transparency = 1
        .Fill.Transparency = 1
    End With
End Function

Public Function LockSides(WallName As String)
    Dim Shp As Shape
    For Each Shp In cSlide.Shapes.Range
        If (Shp.Width = 960 And Shp.Height = 0.2 Or Shp.Width = 0.2 And Shp.Height = 540) And (Shp.Top = 0 Or Shp.Top = 540) And (Shp.Left = 0 Or Shp.Left = 960) And Shp.Name = WallName Then Shp.Delete
    Next
    CreateWall 0, 0, 960, 0.2, WallName: CreateWall 960, 0, 0.2, 540, WallName: CreateWall 0, 540, 960, 0.2, WallName: CreateWall 0, 0, 0.2, 540, WallName
End Function

Function AddCollision(ParamArray Names() As Variant)

    Dim Name As Variant
    For Each Name In Names
        InsertManyColl CStr(Name), cSlide
    Next

End Function

Function InsertOneColl(ByVal Object As Object, ByVal offsetX As Double, ByVal OffSetY As Double, ByVal Width As Double, ByVal Height As Double)

    Objects(Object) = Array(offsetX, OffSetY, Width, Height)

End Function

Private Function InsertManyColl(ByVal DefaultName As String, ByVal Slide As Slide)

    Dim Shape As Shape
    For Each Shape In Slide.Shapes.Range
        If Shape.Name = DefaultName Then
            InsertOneColl Shape, 0, 0, Shape.Width, Shape.Height
        End If
    Next

End Function

Function RemoveAllColl(): Objects.RemoveAll: End Function

Function RemoveOneColl(ByVal Object As Object)

    If Objects.Exists(Object) Then Objects.Remove Object

End Function

Function RemoveManyColl(ByVal DefaultName As String)

    Dim Object As Variant
    For Each Object In Objects
        If Object.Name = DefaultName Then
            RemoveOneColl Object
        End If
    Next

End Function

Function Interset(ByVal Left1 As Double, ByVal Top1 As Double, ByVal Width1 As Double, ByVal Height1 As Double, ByVal Left2 As Double, ByVal Top2 As Double, ByVal Width2 As Double, ByVal Height2 As Double) As Boolean
    Interset = (Left1 + Width1 >= Left2) And (Left1 <= Left2 + Width2) And (Top1 + Height1 >= Top2) And (Top1 <= Top2 + Height2)
End Function

Function IncrementLeft(ByVal Object As Object, ByVal value As Double)

    Dim ObjectBox As Variant, Item As Variant, ItemBox As Variant, LastItem As Object
        ObjectBox = CollisionBox(Object)

    If value > 0 Then

        ObjectBox(2) = ObjectBox(2) + value

        For Each Item In Objects
            If Not Item Is Object Then
                ItemBox = CollisionBox(Item)
                If Interset(ObjectBox(0), ObjectBox(1), ObjectBox(2), ObjectBox(3), ItemBox(0), ItemBox(1), ItemBox(2), ItemBox(3)) Then
                    ObjectBox(2) = ItemBox(0) - ObjectBox(0) - 0.001
                    Set LastItem = Item
                End If
            End If
        Next

        Object.Left = ObjectBox(0) + ObjectBox(2) - Objects(Object)(2) - Objects(Object)(0) - 0.001
        ExecuteHandler Object, LastItem, CollisionDirectionRight

    ElseIf value < 0 Then

        ObjectBox(0) = ObjectBox(0) - Abs(value)
        ObjectBox(2) = ObjectBox(2) + Abs(value)

        For Each Item In Objects
            If Not Item Is Object Then
                ItemBox = CollisionBox(Item)
                If Interset(ObjectBox(0), ObjectBox(1), ObjectBox(2), ObjectBox(3), ItemBox(0), ItemBox(1), ItemBox(2), ItemBox(3)) Then
                    ObjectBox(0) = ItemBox(0) + ItemBox(2) + 0.001
                    ObjectBox(2) = Object.Left - ItemBox(0) + ItemBox(2) - 0.002
                    Set LastItem = Item
                End If
            End If
        Next

        Object.Left = ObjectBox(0) - Objects(Object)(0) + 0.001
        ExecuteHandler Object, LastItem, CollisionDirectionLeft

    End If

End Function

Function SetLeft(ByVal Object As Object, ByVal value As Double)
    IncrementLeft Object, value - Object.Left
End Function

Function IncrementTop(ByVal Object As Object, ByVal value As Double)

    Dim ObjectBox As Variant, Item As Variant, ItemBox As Variant, LastItem As Object
        ObjectBox = CollisionBox(Object)

    If value > 0 Then

        ObjectBox(3) = ObjectBox(3) + value

        For Each Item In Objects
            If Not Item Is Object Then
                ItemBox = CollisionBox(Item)
                If Interset(ObjectBox(0), ObjectBox(1), ObjectBox(2), ObjectBox(3), ItemBox(0), ItemBox(1), ItemBox(2), ItemBox(3)) Then
                    ObjectBox(3) = ItemBox(1) - ObjectBox(1) - 0.001
                    Set LastItem = Item
                End If
            End If
        Next

        Object.Top = ObjectBox(1) + ObjectBox(3) - Objects(Object)(3) - Objects(Object)(1) - 0.001
        ExecuteHandler Object, LastItem, CollisionDirectionDown

    ElseIf value < 0 Then

        ObjectBox(1) = ObjectBox(1) - Abs(value)
        ObjectBox(3) = ObjectBox(3) + Abs(value)

        For Each Item In Objects
            If Not Item Is Object Then
                ItemBox = CollisionBox(Item)
                If Interset(ObjectBox(0), ObjectBox(1), ObjectBox(2), ObjectBox(3), ItemBox(0), ItemBox(1), ItemBox(2), ItemBox(3)) Then
                    ObjectBox(1) = ItemBox(1) + ItemBox(3) + 0.0001
                    ObjectBox(3) = Object.Top - ItemBox(1) + ItemBox(3) - 0.002
                    Set LastItem = Item
                End If
            End If
        Next

        Object.Top = ObjectBox(1) - Objects(Object)(1) + 0.001
        ExecuteHandler Object, LastItem, CollisionDirectionUp

    End If

End Function

Function SetTop(ByVal Object As Object, ByVal value As Double)
    IncrementTop Object, value - Object.Top
End Function

Private Function CollisionBox(ByVal Object As Object) As Variant

    Dim Box As Variant
        Box = Objects(Object)
        Box(0) = Box(0) + Object.Left
        Box(1) = Box(1) + Object.Top

    CollisionBox = Box

End Function

Private Function ExecuteHandler(MovedObject As Object, CollidedObject As Object, Direction As CollisionDirection)
    If Not Handler Is Nothing And Not CollidedObject Is Nothing Then
        Handler.OnCollide MovedObject, CollidedObject, Direction
    End If
End Function
