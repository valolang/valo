VERSION 1.0 CLASS
BEGIN
  MultiUse = -1  'True
END
Attribute VB_Name = "List"
Attribute VB_GlobalNameSpace = False
Attribute VB_Creatable = False
Attribute VB_PredeclaredId = False
Attribute VB_Exposed = False
Option Explicit

Private pItems() As Variant
Private pCount As Long
Private pCapacity As Long

Private Sub Class_Initialize()
    pCapacity = 8
    pCount = 0
    ReDim pItems(0 To pCapacity - 1)
End Sub

Private Sub Class_Terminate()
    Erase pItems
    pCapacity = 0
    pCount = 0
End Sub

' ============================== propriedades ==============================

Public Property Get length() As Long
    length = pCount
End Property

Public Property Get Capacity() As Long
    Capacity = pCapacity
End Property

Public Property Let Capacity(ByVal newCap As Long)
    If newCap < 0 Then Err.Raise 5, "List", "Capacity must be >= 0"
    If newCap < pCount Then Err.Raise 5, "List", "Capacity cannot be less than Count"
    If newCap <> pCapacity Then
        pCapacity = IIf(newCap < 8, 8, newCap)
        ReDim Preserve pItems(0 To pCapacity - 1)
    End If
End Property

Public Function IsEmpty() As Boolean
    IsEmpty = (pCount = 0)
End Function

Public Function At(ByVal Index As Long) As Variant
    If Index < 0 Or Index >= pCount Then Err.Raise 9, "List", "Subscript out of range"
    If IsObject(pItems(Index)) Then
        Set At = pItems(Index)
    Else
        At = pItems(Index)
    End If
End Function

Public Property Get Item(ByVal Index As Long) As Variant
Attribute Item.VB_UserMemId = 0
    If Index < 0 Or Index >= pCount Then Err.Raise 9, "List", "Subscript out of range"
    If IsObject(pItems(Index)) Then
        Set Item = pItems(Index)
    Else
        Item = pItems(Index)
    End If
End Property

Public Property Let Item(ByVal Index As Long, ByVal value As Variant)
Attribute Item.VB_UserMemId = 0
    If Index < 0 Or Index >= pCount Then Err.Raise 9, "List", "Subscript out of range"
    If IsObject(value) Then
        Set pItems(Index) = value
    Else
        pItems(Index) = value
    End If
End Property

Public Property Set Item(ByVal Index As Long, ByVal value As Object)
    If Index < 0 Or Index >= pCount Then Err.Raise 9, "List", "Subscript out of range"
    Set pItems(Index) = value
End Property

Public Property Get Items() As Variant
    If pCount = 0 Then
        Dim arr() As Variant
        ReDim arr(-1 To -1)
        Items = arr
    Else
        If pCount < pCapacity Then
            ReDim Preserve pItems(0 To pCount - 1)
            Items = pItems
            ReDim Preserve pItems(0 To pCapacity - 1)
        Else
            Items = pItems
        End If
    End If
End Property

Public Property Let Items(ByVal newItems As Variant)
    If Not IsArray(newItems) Then Err.Raise 5, "List", "Assigned value must be a 1-D array"
    Dim lb As Long, ub As Long, cnt As Long
    lb = LBound(newItems, 1)
    ub = UBound(newItems, 1)
    cnt = ub - lb + 1
    
    pCount = cnt
    pCapacity = IIf(cnt < 8, 8, cnt)
    ReDim pItems(0 To pCapacity - 1)
    
    Dim i As Long
    For i = 0 To cnt - 1
        If IsObject(newItems(lb + i)) Then
            Set pItems(i) = newItems(lb + i)
        Else
            pItems(i) = newItems(lb + i)
        End If
    Next i
End Property

' ============================== operações básicas ==============================

Public Sub Add(ByVal value As Variant)
    If pCount >= pCapacity Then
        pCapacity = pCapacity * 2
        If pCapacity < 8 Then pCapacity = 8
        ReDim Preserve pItems(0 To pCapacity - 1)
    End If
    If IsObject(value) Then
        Set pItems(pCount) = value
    Else
        pItems(pCount) = value
    End If
    pCount = pCount + 1
End Sub

Public Sub AddRange(ByVal source As Variant)
    Dim arr As Variant, lb As Long, ub As Long, n As Long
    If IsObject(source) Then
        If TypeOf source Is List Then
            arr = source.ToArray
        Else
            Err.Raise 5, "List", "AddRange accepts a 1-D array or a List"
        End If
    ElseIf IsArray(source) Then
        arr = source
    Else
        Err.Raise 5, "List", "AddRange requires a 1-D array or List"
    End If
    
    lb = LBound(arr, 1): ub = UBound(arr, 1): n = ub - lb + 1
    If n <= 0 Then Exit Sub
    
    Dim required As Long
    required = pCount + n
    If required > pCapacity Then
        Do While pCapacity < required
            pCapacity = pCapacity * 2
            If pCapacity < 8 Then pCapacity = 8
        Loop
        ReDim Preserve pItems(0 To pCapacity - 1)
    End If
    
    Dim i As Long, v As Variant
    For i = 0 To n - 1
        v = arr(lb + i)
        If IsObject(v) Then
            Set pItems(pCount + i) = v
        Else
            pItems(pCount + i) = v
        End If
    Next i
    pCount = pCount + n
End Sub

Public Sub InsertAt(ByVal Index As Long, ByVal value As Variant)
    If Index < 0 Or Index > pCount Then Err.Raise 9, "List", "Subscript out of range"
    If pCount >= pCapacity Then
        pCapacity = pCapacity * 2
        If pCapacity < 8 Then pCapacity = 8
        ReDim Preserve pItems(0 To pCapacity - 1)
    End If
    
    Dim i As Long
    For i = pCount To Index + 1 Step -1
        If IsObject(pItems(i - 1)) Then
            Set pItems(i) = pItems(i - 1)
        Else
            pItems(i) = pItems(i - 1)
        End If
    Next i
    
    If IsObject(value) Then
        Set pItems(Index) = value
    Else
        pItems(Index) = value
    End If
    pCount = pCount + 1
End Sub

Public Sub InsertRange(ByVal Index As Long, ByVal source As Variant)
    If Index < 0 Or Index > pCount Then Err.Raise 9, "List", "Subscript out of range"
    Dim arr As Variant, lb As Long, ub As Long, n As Long
    If IsObject(source) Then
        If TypeOf source Is List Then
            arr = source.ToArray
        Else
            Err.Raise 5, "List", "InsertRange accepts a 1-D array or a List"
        End If
    ElseIf IsArray(source) Then
        arr = source
    Else
        Err.Raise 5, "List", "InsertRange requires a 1-D array or List"
    End If
    
    lb = LBound(arr, 1): ub = UBound(arr, 1): n = ub - lb + 1
    If n <= 0 Then Exit Sub
    
    Dim required As Long
    required = pCount + n
    If required > pCapacity Then
        Do While pCapacity < required
            pCapacity = pCapacity * 2
            If pCapacity < 8 Then pCapacity = 8
        Loop
        ReDim Preserve pItems(0 To pCapacity - 1)
    End If
    
    Dim i As Long, v As Variant
    For i = pCount - 1 To Index Step -1
        If IsObject(pItems(i)) Then
            Set pItems(i + n) = pItems(i)
        Else
            pItems(i + n) = pItems(i)
        End If
    Next i
    
    For i = 0 To n - 1
        v = arr(lb + i)
        If IsObject(v) Then
            Set pItems(Index + i) = v
        Else
            pItems(Index + i) = v
        End If
    Next i
    
    pCount = pCount + n
End Sub

Public Sub RemoveAt(ByVal Index As Long)
    If Index < 0 Or Index >= pCount Then Err.Raise 9, "List", "Subscript out of range"
    Dim i As Long
    For i = Index To pCount - 2
        If IsObject(pItems(i + 1)) Then
            Set pItems(i) = pItems(i + 1)
        Else
            pItems(i) = pItems(i + 1)
        End If
    Next i
    If IsObject(pItems(pCount - 1)) Then
        Set pItems(pCount - 1) = Nothing
    Else
        pItems(pCount - 1) = Empty
    End If
    pCount = pCount - 1
End Sub

Public Function Remove(ByVal value As Variant) As Boolean
    Dim idx As Long
    idx = IndexOf(value)
    If idx <> -1 Then
        RemoveAt idx
        Remove = True
    End If
End Function

Public Function Pop() As Variant
    If pCount = 0 Then Err.Raise 9, "List", "List is empty"
    Dim idx As Long: idx = pCount - 1
    If IsObject(pItems(idx)) Then
        Set Pop = pItems(idx)
        Set pItems(idx) = Nothing
    Else
        Pop = pItems(idx)
        pItems(idx) = Empty
    End If
    pCount = pCount - 1
End Function

Public Function shift() As Variant
    If pCount = 0 Then Err.Raise 9, "List", "List is empty"
    If IsObject(pItems(0)) Then
        Set shift = pItems(0)
    Else
        shift = pItems(0)
    End If
    RemoveAt 0
End Function

Public Sub Clear()
    Erase pItems
    pCapacity = 8
    pCount = 0
    ReDim pItems(0 To pCapacity - 1)
End Sub

Public Sub Trim()
    If pCount <= 0 Then
        Clear
    Else
        pCapacity = pCount
        ReDim Preserve pItems(0 To pCapacity - 1)
    End If
End Sub

Public Sub Reserve(ByVal additionalCapacity As Long)
    If additionalCapacity < 0 Then Err.Raise 5, "List", "additionalCapacity must be >= 0"
    Dim required As Long
    required = pCount + additionalCapacity
    If required > pCapacity Then
        Do While pCapacity < required
            pCapacity = pCapacity * 2
            If pCapacity < 8 Then pCapacity = 8
        Loop
        ReDim Preserve pItems(0 To pCapacity - 1)
    End If
End Sub

' ============================== utilitários ==============================

Public Sub Shuffle()
    Dim i As Long, j As Long
    Randomize
    For i = pCount - 1 To 1 Step -1
        j = Int((i + 1) * Rnd)
        Swap i, j
    Next i
End Sub

Public Function Join(Optional ByVal delimiter As String = ",") As String
    If pCount = 0 Then
        Join = ""
        Exit Function
    End If
    Dim s() As String
    ReDim s(0 To pCount - 1)
    Dim i As Long, v As Variant
    For i = 0 To pCount - 1
        v = pItems(i)
        If IsObject(v) Then
            s(i) = TypeName(v)
        ElseIf IsError(v) Then
            s(i) = "#Error"
        ElseIf IsNull(v) Then
            s(i) = ""
        Else
            s(i) = CStr(v)
        End If
    Next i
    Join = VBA.Join(s, delimiter)
End Function

Public Sub Reverse()
    Dim i As Long, j As Long
    j = pCount - 1
    For i = 0 To (pCount \ 2) - 1
        Swap i, j
        j = j - 1
    Next i
End Sub

Public Function ToArray() As Variant
    ToArray = Me.Items
End Function

' ============================== buscas ==============================

Public Function IndexOf(ByVal value As Variant, Optional ByVal startIndex As Long = 0, Optional ByVal textCompare As VbCompareMethod = vbBinaryCompare) As Long
    If startIndex < 0 Then startIndex = 0
    Dim i As Long
     
    If IsObject(value) Then
        For i = startIndex To pCount - 1
            If IsObject(pItems(i)) Then
                If pItems(i) Is value Then
                    IndexOf = i
                    Exit Function
                End If
            End If
        Next i
    ElseIf VarType(value) = vbString And textCompare <> vbBinaryCompare Then
        For i = startIndex To pCount - 1
            If VarType(pItems(i)) = vbString Then
                If StrComp(CStr(pItems(i)), CStr(value), textCompare) = 0 Then
                    IndexOf = i
                    Exit Function
                End If
            End If
        Next i
    Else
        For i = startIndex To pCount - 1
            If Not IsObject(pItems(i)) Then
                If pItems(i) = value Then
                    IndexOf = i
                    Exit Function
                End If
            End If
        Next i
    End If
    IndexOf = -1
End Function

Public Function LastIndexOf(ByVal value As Variant, Optional ByVal startIndex As Long = -1, Optional ByVal textCompare As VbCompareMethod = vbBinaryCompare) As Long
    Dim i As Long
    If startIndex < 0 Or startIndex >= pCount Then startIndex = pCount - 1
    
    If IsObject(value) Then
        For i = startIndex To 0 Step -1
            If IsObject(pItems(i)) Then
                If pItems(i) Is value Then
                    LastIndexOf = i
                    Exit Function
                End If
            End If
        Next i
    ElseIf VarType(value) = vbString And textCompare <> vbBinaryCompare Then
        For i = startIndex To 0 Step -1
            If VarType(pItems(i)) = vbString Then
                If StrComp(CStr(pItems(i)), CStr(value), textCompare) = 0 Then
                    LastIndexOf = i
                    Exit Function
                End If
            End If
        Next i
    Else
        For i = startIndex To 0 Step -1
            If Not IsObject(pItems(i)) Then
                If pItems(i) = value Then
                    LastIndexOf = i
                    Exit Function
                End If
            End If
        Next i
    End If
    LastIndexOf = -1
End Function

Public Function Contains(ByVal value As Variant, Optional ByVal textCompare As VbCompareMethod = vbBinaryCompare) As Boolean
    Contains = (IndexOf(value, 0, textCompare) <> -1)
End Function

Public Function Includes(ByVal value As Variant, Optional ByVal textCompare As VbCompareMethod = vbBinaryCompare) As Boolean
    Includes = (IndexOf(value, 0, textCompare) <> -1)
End Function

Public Function BinarySearch(ByVal value As Variant, Optional ByVal ascending As Boolean = True, Optional ByVal textCompare As VbCompareMethod = vbBinaryCompare, Optional ByVal returnInsertionPoint As Boolean = False) As Long
    If pCount = 0 Then
        BinarySearch = IIf(returnInsertionPoint, -1, -1)
        Exit Function
    End If
    Dim i As Long
    For i = 0 To pCount - 1
        If IsObject(pItems(i)) Then Err.Raise 5, "List", "BinarySearch does not support objects"
    Next i
    
    Dim low As Long, high As Long, mid As Long, cmp As Long
    low = 0: high = pCount - 1
    
    Do While low <= high
        mid = (low + high) \ 2
        cmp = CompareValues(pItems(mid), value, textCompare)
        If cmp = 0 Then
            BinarySearch = mid
            Exit Function
        End If
        
        If ascending Then
            If cmp < 0 Then low = mid + 1 Else high = mid - 1
        Else
            If cmp > 0 Then low = mid + 1 Else high = mid - 1
        End If
    Loop
    
    If returnInsertionPoint Then BinarySearch = -(low + 1) Else BinarySearch = -1
End Function

' ============================== higher-order ==============================

Public Sub ForEach(ByVal callback As Object)
    Dim i As Long
    For i = 0 To pCount - 1
        CallByName callback, "Invoke", VbMethod, pItems(i)
    Next i
End Sub

Public Function Every(ByVal callback As Object) As Boolean
    Dim i As Long
    For i = 0 To pCount - 1
        If Not CallByName(callback, "Invoke", VbMethod, pItems(i)) Then
            Every = False: Exit Function
        End If
    Next i
    Every = True
End Function

Public Function Some(ByVal callback As Object) As Boolean
    Dim i As Long
    For i = 0 To pCount - 1
        If CallByName(callback, "Invoke", VbMethod, pItems(i)) Then
            Some = True: Exit Function
        End If
    Next i
    Some = False
End Function

Public Function Map(ByVal callback As Object) As List
    Dim Result As New List, i As Long, mapped As Variant
    For i = 0 To pCount - 1
        mapped = CallByName(callback, "Invoke", VbMethod, pItems(i))
        Result.Add mapped
    Next i
    Set Map = Result
End Function

Public Function Filter(ByVal callback As Object) As List
    Dim Result As New List, i As Long, keep As Boolean
    For i = 0 To pCount - 1
        keep = CallByName(callback, "Invoke", VbMethod, pItems(i))
        If keep Then Result.Add pItems(i)
    Next i
    Set Filter = Result
End Function

Public Function Reduce(ByVal callback As Object, ByVal initial As Variant) As Variant
    Dim acc As Variant: acc = initial
    Dim i As Long
    For i = 0 To pCount - 1
        acc = CallByName(callback, "Invoke", VbMethod, acc, pItems(i))
    Next i
    Reduce = acc
End Function

Public Function FindAll(ByVal callback As Object) As List
    Set FindAll = Filter(callback)
End Function

Public Function Slice(ByVal startIndex As Long, ByVal endIndex As Long) As List
    Dim Result As New List, i As Long
    If startIndex < 0 Then startIndex = 0
    If endIndex > pCount Then endIndex = pCount
    If endIndex < startIndex Then
        Set Slice = Result
        Exit Function
    End If
    For i = startIndex To endIndex - 1
        Result.Add pItems(i)
    Next i
    Set Slice = Result
End Function

Public Function Concat(ByVal other As Variant) As List
    Dim Result As New List
    Result.AddRange Me.Items
    
    If IsObject(other) Then
        If TypeOf other Is List Then
            Result.AddRange other.Items
        Else
            Err.Raise 5, "List", "Concat accepts a List or a 1-D array"
        End If
    ElseIf IsArray(other) Then
        Result.AddRange other
    Else
        Err.Raise 5, "List", "Concat accepts a List or a 1-D array"
    End If
    Set Concat = Result
End Function

Public Function Clone() As List
    Dim Result As New List
    Result.AddRange Me.Items
    Set Clone = Result
End Function

' ============================== ordenação ==============================

Private Sub Swap(ByVal i As Long, ByVal j As Long)
    If i = j Then Exit Sub
    Dim tmp As Variant
    If IsObject(pItems(i)) Then Set tmp = pItems(i) Else tmp = pItems(i)
    If IsObject(pItems(j)) Then Set pItems(i) = pItems(j) Else pItems(i) = pItems(j)
    If IsObject(tmp) Then Set pItems(j) = tmp Else pItems(j) = tmp
End Sub

Private Function CompareValues(ByVal a As Variant, ByVal b As Variant, ByVal textCompare As VbCompareMethod) As Long
    If IsNull(a) And IsNull(b) Then CompareValues = 0: Exit Function
    If IsNull(a) Then CompareValues = -1: Exit Function
    If IsNull(b) Then CompareValues = 1: Exit Function
    
    If VarType(a) = vbString And VarType(b) = vbString Then
        CompareValues = Sgn(StrComp(CStr(a), CStr(b), textCompare))
        Exit Function
    End If
    
    If a = b Then CompareValues = 0: Exit Function
    If a < b Then CompareValues = -1 Else CompareValues = 1
End Function

Private Sub InsertionSort(ByVal low As Long, ByVal high As Long, ByVal asc As Boolean, ByVal textCompare As VbCompareMethod)
    Dim i As Long, j As Long, key As Variant
    For i = low + 1 To high
        If IsObject(pItems(i)) Then Set key = pItems(i) Else key = pItems(i)
        j = i - 1
        If asc Then
            Do While j >= low
                If CompareValues(pItems(j), key, textCompare) > 0 Then
                    If IsObject(pItems(j)) Then Set pItems(j + 1) = pItems(j) Else pItems(j + 1) = pItems(j)
                    j = j - 1
                Else
                    Exit Do
                End If
            Loop
        Else
            Do While j >= low
                If CompareValues(pItems(j), key, textCompare) < 0 Then
                    If IsObject(pItems(j)) Then Set pItems(j + 1) = pItems(j) Else pItems(j + 1) = pItems(j)
                    j = j - 1
                Else
                    Exit Do
                End If
            Loop
        End If
        If IsObject(key) Then Set pItems(j + 1) = key Else pItems(j + 1) = key
    Next i
End Sub

Public Sub Sort(Optional ByVal ascending As Boolean = True, Optional ByVal textCompare As VbCompareMethod = vbBinaryCompare)
    If pCount < 2 Then Exit Sub
    Dim i As Long
    For i = 0 To pCount - 1
        If IsObject(pItems(i)) Then Err.Raise 5, "List", "Sort does not support objects"
    Next i
    QuickSort 0, pCount - 1, ascending, 0, textCompare
End Sub

Private Sub QuickSort(ByVal low As Long, ByVal high As Long, ByVal asc As Boolean, ByVal depth As Long, ByVal textCompare As VbCompareMethod)
    Const MAX_DEPTH As Long = 64
    If high - low < 16 Then
        InsertionSort low, high, asc, textCompare
        Exit Sub
    End If
    If depth > MAX_DEPTH Then
        InsertionSort low, high, asc, textCompare
        Exit Sub
    End If
    
    Dim i As Long, j As Long, pivot As Variant
    i = low: j = high: pivot = pItems((low + high) \ 2)
    
    Do While i <= j
        If asc Then
            Do While CompareValues(pItems(i), pivot, textCompare) < 0: i = i + 1: Loop
            Do While CompareValues(pItems(j), pivot, textCompare) > 0: j = j - 1: Loop
        Else
            Do While CompareValues(pItems(i), pivot, textCompare) > 0: i = i + 1: Loop
            Do While CompareValues(pItems(j), pivot, textCompare) < 0: j = j - 1: Loop
        End If
        If i <= j Then
            Swap i, j
            i = i + 1
            j = j - 1
        End If
    Loop
    
    If low < j Then QuickSort low, j, asc, depth + 1, textCompare
    If i < high Then QuickSort i, high, asc, depth + 1, textCompare
End Sub
