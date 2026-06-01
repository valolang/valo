Attribute VB_Name = "JSON"
Option Explicit

Public Type JsonOptions
    AllowUnquotedKeys As Boolean
    AllowSingleQuotes As Boolean
End Type

Public Options As JsonOptions

Public Function ParseJson(ByVal Text As String) As Variant
    Dim p As Long
    Dim result As Variant
    p = 1
    SkipWS Text, p
    Assign result, ParseValue(Text, p)
    SkipWS Text, p
    If p <= Len(Text) Then Fail "Trailing data", Text, p
    If IsObject(result) Then
        Set ParseJson = result
    Else
        ParseJson = result
    End If
End Function

Private Function ParseValue(ByVal s As String, ByRef p As Long) As Variant
    Dim ch As String
    If p > Len(s) Then Fail "Unexpected end", s, p
    ch = Mid$(s, p, 1)
    Select Case ch
        Case "{"
            Set ParseValue = ParseObject(s, p)
        Case "["
            Set ParseValue = ParseArray(s, p)
        Case """"
            ParseValue = ParseString(s, p, """")
        Case "'"
            If Options.AllowSingleQuotes Then
                ParseValue = ParseString(s, p, "'")
            Else
                Fail "Single quotes not allowed", s, p
            End If
        Case "t"
            ExpectLiteral s, p, "true"
            ParseValue = True
        Case "f"
            ExpectLiteral s, p, "false"
            ParseValue = False
        Case "n"
            ExpectLiteral s, p, "null"
            ParseValue = Null
        Case "-", "0", "1", "2", "3", "4", "5", "6", "7", "8", "9"
            ParseValue = ParseNumber(s, p)
        Case Else
            If Options.AllowUnquotedKeys Then
                ParseValue = ParseIdentifierValue(s, p)
            Else
                Fail "Unexpected token", s, p
            End If
    End Select
End Function

Private Sub Assign(ByRef target As Variant, ByVal source As Variant)
    If IsObject(source) Then
        Set target = source
    Else
        target = source
    End If
End Sub

Private Function ParseObject(ByVal s As String, ByRef p As Long) As Object
    Dim d As Object
    Set d = CreateObject("Scripting.Dictionary")
    p = p + 1
    SkipWS s, p
    If p <= Len(s) Then
        If Mid$(s, p, 1) = "}" Then
            p = p + 1
            Set ParseObject = d
            Exit Function
        End If
    Else
        Fail "Unexpected end", s, p
    End If
    
    Do
        Dim k As String
        k = ParseKey(s, p)
        SkipWS s, p
        If p > Len(s) Then Fail "Unexpected end", s, p
        If Mid$(s, p, 1) <> ":" Then Fail "Expected ':'", s, p
        p = p + 1
        SkipWS s, p
        Dim v As Variant
        Assign v, ParseValue(s, p)
        If d.Exists(k) Then Fail "Duplicate key", s, p
        If IsObject(v) Then
            d.Add k, v
        Else
            d.Add k, v
        End If
        SkipWS s, p
        If p > Len(s) Then Fail "Unexpected end", s, p
        Dim ch As String
        ch = Mid$(s, p, 1)
        If ch = "}" Then
            p = p + 1
            Exit Do
        ElseIf ch = "," Then
            p = p + 1
            SkipWS s, p
        Else
            Fail "Expected ',' or '}'", s, p
        End If
    Loop
    
    Set ParseObject = d
End Function

Private Function ParseArray(ByVal s As String, ByRef p As Long) As Object
    Dim a As Object
    Set a = New List
    p = p + 1
    SkipWS s, p
    If p <= Len(s) Then
        If Mid$(s, p, 1) = "]" Then
            p = p + 1
            Set ParseArray = a
            Exit Function
        End If
    Else
        Fail "Unexpected end", s, p
    End If
    
    Do
        Dim v As Variant
        Assign v, ParseValue(s, p)
        If IsObject(v) Then
            a.Add v
        Else
            a.Add v
        End If
        SkipWS s, p
        If p > Len(s) Then Fail "Unexpected end", s, p
        Dim ch As String
        ch = Mid$(s, p, 1)
        If ch = "]" Then
            p = p + 1
            Exit Do
        ElseIf ch = "," Then
            p = p + 1
            SkipWS s, p
        Else
            Fail "Expected ',' or ']'", s, p
        End If
    Loop
    
    Set ParseArray = a
End Function

Private Function ParseKey(ByVal s As String, ByRef p As Long) As String
    SkipWS s, p
    If p > Len(s) Then Fail "Unexpected end", s, p
    Dim ch As String
    ch = Mid$(s, p, 1)
    If ch = """" Then
        ParseKey = ParseString(s, p, """")
        Exit Function
    End If
    If ch = "'" Then
        If Options.AllowSingleQuotes Then
            ParseKey = ParseString(s, p, "'")
            Exit Function
        End If
        Fail "Single quotes not allowed", s, p
    End If
    If Options.AllowUnquotedKeys Then
        ParseKey = ParseIdentifierKey(s, p)
        Exit Function
    End If
    Fail "Expected string key", s, p
End Function

Private Function ParseString(ByVal s As String, ByRef p As Long, ByVal quote As String) As String
    Dim out As String
    p = p + 1
    Do While p <= Len(s)
        Dim ch As String
        ch = Mid$(s, p, 1)
        If ch = quote Then
            p = p + 1
            ParseString = out
            Exit Function
        End If
        If ch = "\" Then
            p = p + 1
            If p > Len(s) Then Fail "Unterminated escape", s, p
            ch = Mid$(s, p, 1)
            Select Case ch
                Case """": out = out & """"
                Case "\": out = out & "\"
                Case "/": out = out & "/"
                Case "b": out = out & Chr$(8)
                Case "f": out = out & Chr$(12)
                Case "n": out = out & vbLf
                Case "r": out = out & vbCr
                Case "t": out = out & vbTab
                Case "u"
                    Dim cp As Long
                    cp = ParseHex4(s, p)
                    out = out & UnicodeFromCodepoint(cp)
                Case Else
                    Fail "Invalid escape", s, p
            End Select
            p = p + 1
        Else
            Dim c As Integer
            c = AscW(ch)
            If c >= 0 And c < 32 Then Fail "Control char in string", s, p
            out = out & ch
            p = p + 1
        End If
    Loop
    Fail "Unterminated string", s, p
End Function

Private Function ParseHex4(ByVal s As String, ByRef p As Long) As Long
    Dim i As Long, v As Long, d As Long
    v = 0
    For i = 1 To 4
        p = p + 1
        If p > Len(s) Then Fail "Invalid \u escape", s, p
        d = HexVal(Mid$(s, p, 1))
        If d < 0 Then Fail "Invalid hex in \u escape", s, p
        v = (v * 16) + d
    Next i
    ParseHex4 = v
End Function

Private Function HexVal(ByVal ch As String) As Long
    Dim c As Long
    c = AscW(ch)
    If c >= 48 And c <= 57 Then
        HexVal = c - 48
    ElseIf c >= 65 And c <= 70 Then
        HexVal = c - 55
    ElseIf c >= 97 And c <= 102 Then
        HexVal = c - 87
    Else
        HexVal = -1
    End If
End Function

Private Function UnicodeFromCodepoint(ByVal cp As Long) As String
    If cp >= 0 And cp <= 65535 Then
        UnicodeFromCodepoint = ChrW$(cp)
    Else
        UnicodeFromCodepoint = ""
    End If
End Function

Private Function ParseNumber(ByVal s As String, ByRef p As Long) As Variant
    Dim start As Long
    start = p
    Dim ch As String
    
    ch = Mid$(s, p, 1)
    If ch = "-" Then
        p = p + 1
        If p > Len(s) Then Fail "Invalid number", s, p
    End If
    
    ch = Mid$(s, p, 1)
    If ch = "0" Then
        p = p + 1
    ElseIf ch >= "1" And ch <= "9" Then
        Do While p <= Len(s)
            ch = Mid$(s, p, 1)
            If ch < "0" Or ch > "9" Then Exit Do
            p = p + 1
        Loop
    Else
        Fail "Invalid number", s, p
    End If
    
    If p <= Len(s) Then
        ch = Mid$(s, p, 1)
        If ch = "." Then
            p = p + 1
            If p > Len(s) Then Fail "Invalid number", s, p
            ch = Mid$(s, p, 1)
            If ch < "0" Or ch > "9" Then Fail "Invalid number", s, p
            Do While p <= Len(s)
                ch = Mid$(s, p, 1)
                If ch < "0" Or ch > "9" Then Exit Do
                p = p + 1
            Loop
        End If
    End If
    
    If p <= Len(s) Then
        ch = Mid$(s, p, 1)
        If ch = "e" Or ch = "E" Then
            p = p + 1
            If p > Len(s) Then Fail "Invalid exponent", s, p
            ch = Mid$(s, p, 1)
            If ch = "+" Or ch = "-" Then
                p = p + 1
                If p > Len(s) Then Fail "Invalid exponent", s, p
            End If
            ch = Mid$(s, p, 1)
            If ch < "0" Or ch > "9" Then Fail "Invalid exponent", s, p
            Do While p <= Len(s)
                ch = Mid$(s, p, 1)
                If ch < "0" Or ch > "9" Then Exit Do
                p = p + 1
            Loop
        End If
    End If
    
    Dim numText As String
    numText = Mid$(s, start, p - start)
    
    Dim hasDot As Boolean, hasExp As Boolean
    hasDot = (InStr(1, numText, ".", vbBinaryCompare) > 0)
    hasExp = (InStr(1, numText, "e", vbTextCompare) > 0)
    
    If hasDot Or hasExp Then
        ParseNumber = CDbl(Replace$(numText, ".", Format$(0, ".")))
    Else
        Dim ll As Double
        ll = CDbl(numText)
        If ll >= -2147483648# And ll <= 2147483647# Then
            ParseNumber = CLng(ll)
        Else
            ParseNumber = ll
        End If
    End If
End Function

Private Function ParseIdentifierKey(ByVal s As String, ByRef p As Long) As String
    Dim start As Long
    start = p
    Dim ch As String
    ch = Mid$(s, p, 1)
    If Not IsIdentStart(ch) Then Fail "Invalid identifier", s, p
    p = p + 1
    Do While p <= Len(s)
        ch = Mid$(s, p, 1)
        If Not IsIdentPart(ch) Then Exit Do
        p = p + 1
    Loop
    ParseIdentifierKey = Mid$(s, start, p - start)
End Function

Private Function ParseIdentifierValue(ByVal s As String, ByRef p As Long) As Variant
    Dim id As String
    id = ParseIdentifierKey(s, p)
    If LCase$(id) = "true" Then
        ParseIdentifierValue = True
    ElseIf LCase$(id) = "false" Then
        ParseIdentifierValue = False
    ElseIf LCase$(id) = "null" Then
        ParseIdentifierValue = Null
    Else
        ParseIdentifierValue = id
    End If
End Function

Private Function IsIdentStart(ByVal ch As String) As Boolean
    Dim c As Long
    c = AscW(ch)
    IsIdentStart = (c >= 65 And c <= 90) Or (c >= 97 And c <= 122) Or (ch = "_")
End Function

Private Function IsIdentPart(ByVal ch As String) As Boolean
    Dim c As Long
    c = AscW(ch)
    IsIdentPart = (c >= 65 And c <= 90) Or (c >= 97 And c <= 122) Or (c >= 48 And c <= 57) Or (ch = "_")
End Function

Private Sub SkipWS(ByVal s As String, ByRef p As Long)
    Do While p <= Len(s)
        Dim ch As String
        ch = Mid$(s, p, 1)
        If ch = " " Or ch = vbTab Or ch = vbCr Or ch = vbLf Then
            p = p + 1
        Else
            Exit Do
        End If
    Loop
End Sub

Private Sub ExpectLiteral(ByVal s As String, ByRef p As Long, ByVal lit As String)
    If p + Len(lit) - 1 > Len(s) Then Fail "Unexpected end", s, p
    If Mid$(s, p, Len(lit)) <> lit Then Fail "Invalid literal", s, p
    p = p + Len(lit)
End Sub

Private Sub Fail(ByVal msg As String, ByVal s As String, ByVal p As Long)
    Dim line As Long, col As Long, i As Long, ch As String
    line = 1
    col = 1
    For i = 1 To p - 1
        ch = Mid$(s, i, 1)
        If ch = vbLf Then
            line = line + 1
            col = 1
        ElseIf ch = vbCr Then
            line = line + 1
            col = 1
            If i < Len(s) Then
                If Mid$(s, i + 1, 1) = vbLf Then i = i + 1
            End If
        Else
            col = col + 1
        End If
    Next i
    Err.Raise vbObjectError + 513, "FastJson", msg & " at line " & CStr(line) & " col " & CStr(col)
End Sub

Public Function StringifyJson(ByVal v As Variant, Optional ByVal Indentation As Long = 0) As String
    Dim ind As String
    If Indentation <= 0 Then
        ind = ""
    Else
        ind = String$(Indentation, " ")
    End If
    StringifyJson = WriteValue(v, ind, 0)
End Function

Private Function WriteValue(ByVal v As Variant, ByVal ind As String, ByVal depth As Long) As String
    If IsObject(v) Then
        Dim tName As String
        tName = typeName(v)
        If tName = "Dictionary" Or InStr(1, tName, "Dictionary") > 0 Then
            WriteValue = WriteObject(v, ind, depth)
            Exit Function
        End If
        If tName = "List" Then
            WriteValue = WriteList(v, ind, depth)
            Exit Function
        End If
        If tName = "Nothing" Then
            WriteValue = "null"
            Exit Function
        End If
        Fail "Unsupported object type: " & tName, "", 1
    End If
    
    Select Case VarType(v)
        Case vbEmpty
            WriteValue = "null"
        Case vbNull
            WriteValue = "null"
        Case vbBoolean
            If v Then WriteValue = "true" Else WriteValue = "false"
        Case vbByte, vbInteger, vbLong, vbSingle, vbDouble, vbCurrency, vbDecimal
            WriteValue = Replace$(CStr(v), Format$(0, "."), ".")
        Case vbString
            WriteValue = WriteString(CStr(v))
        Case Else
            If IsArray(v) Then
                WriteValue = WriteVbaArray(v, ind, depth)
            Else
                Fail "Unsupported type: " & typeName(v), "", 1
            End If
    End Select
End Function

Private Function WriteObject(ByVal d As Object, ByVal ind As String, ByVal depth As Long) As String
    Dim i As Long, n As Long
    n = d.Count
    If n = 0 Then
        WriteObject = "{}"
        Exit Function
    End If
    
    Dim nl As String, pad As String, padIn As String
    nl = IIf(ind = "", "", vbLf)
    pad = IIf(ind = "", "", String$(depth * Len(ind), " "))
    padIn = IIf(ind = "", "", String$((depth + 1) * Len(ind), " "))
    
    Dim s As String
    Dim keys As Variant, items As Variant
    keys = d.keys
    items = d.items
    
    s = "{" & nl
    For i = 0 To n - 1
        s = s & padIn & WriteString(CStr(keys(i))) & ":" & IIf(ind = "", "", " ")
        Dim val As Variant
        If IsObject(items(i)) Then
            Set val = items(i)
        Else
            val = items(i)
        End If
        s = s & WriteValue(val, ind, depth + 1)
        If i < n - 1 Then s = s & "," & nl
    Next i
    s = s & nl & pad & "}"
    WriteObject = s
End Function

Private Function WriteList(ByVal a As Object, ByVal ind As String, ByVal depth As Long) As String
    Dim n As Long
    n = a.length
    If n = 0 Then
        WriteList = "[]"
        Exit Function
    End If
    
    Dim nl As String, pad As String, padIn As String
    nl = IIf(ind = "", "", vbLf)
    pad = IIf(ind = "", "", String$(depth * Len(ind), " "))
    padIn = IIf(ind = "", "", String$((depth + 1) * Len(ind), " "))
    
    Dim i As Long, s As String
    s = "[" & nl
    For i = 0 To n - 1
        s = s & padIn & WriteValue(a(i), ind, depth + 1)
        If i < n - 1 Then s = s & "," & nl
    Next i
    s = s & nl & pad & "]"
    WriteList = s
End Function

Private Function WriteVbaArray(ByVal arr As Variant, ByVal ind As String, ByVal depth As Long) As String
    Dim l As Long, u As Long, i As Long
    l = LBound(arr)
    u = UBound(arr)
    If u < l Then
        WriteVbaArray = "[]"
        Exit Function
    End If
    
    Dim nl As String, pad As String, padIn As String
    nl = IIf(ind = "", "", vbLf)
    pad = IIf(ind = "", "", String$(depth * Len(ind), " "))
    padIn = IIf(ind = "", "", String$((depth + 1) * Len(ind), " "))
    
    Dim s As String
    s = "[" & nl
    For i = l To u
        s = s & padIn & WriteValue(arr(i), ind, depth + 1)
        If i < u Then s = s & "," & nl
    Next i
    s = s & nl & pad & "]"
    WriteVbaArray = s
End Function

Private Function WriteString(ByVal t As String) As String
    Dim i As Long, ch As String, c As Long
    Dim out As String
    out = """"
    For i = 1 To Len(t)
        ch = Mid$(t, i, 1)
        c = AscW(ch)
        Select Case ch
            Case """": out = out & "\"""
            Case "\": out = out & "\\"
            Case vbTab: out = out & "\t"
            Case vbCr: out = out & "\r"
            Case vbLf: out = out & "\n"
            Case Else
                If c >= 0 And c < 32 Then
                    out = out & "\u" & Right$("000" & Hex$(c), 4)
                Else
                    out = out & ch
                End If
        End Select
    Next i
    WriteString = out & """"
End Function
