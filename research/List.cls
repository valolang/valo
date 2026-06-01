VERSION 1.0 CLASS
BEGIN
  MultiUse = -1  'True
END
Attribute VB_Name = "List"
Attribute VB_GlobalNameSpace = False
Attribute VB_Creatable = False
Attribute VB_PredeclaredId = False
Attribute VB_Exposed = False
' ******************************************************************************
'  List Class – High-Performance Dynamic Array for VBA
' ******************************************************************************
'
'  Author    : Bedrock Team
'  Last Upd. : 19 Sep 2025
'
'  OVERVIEW
'  ---------------------------------------------------------------------------
'  Dynamic resizable array providing safe indexed access, flexible capacity
'  growth, and extended collection-like operations (Add, Remove, Insert).
'
'  FEATURES
'  ---------------------------------------------------------------------------
'  • Automatic capacity management with overflow protection
'  • Core operations: Add, Insert, Remove, Pop, Shift
'  • Search utilities: IndexOf, LastIndexOf, Contains, BinarySearch
'  • Sorting: QuickSort + InsertionSort (numbers, strings, Null only)
'  • Utility functions: ToArray, Join, Reverse, Shuffle, Slice
'  • Higher-order methods: Map, Filter, Reduce, Every, Some, ForEach
'
'  LIMITATIONS
'  ---------------------------------------------------------------------------
'  • Sorting and BinarySearch do not support object references
'  • Comparisons on incompatible types raise an error
'
' ******************************************************************************

Option Explicit

Private Type TList
    Items() As Variant
    Count As Long
    Capacity As Long
End Type

Private Const DEFAULT_CAP As Long = 8
Private Const GROW_THRESHOLD As Long = 1024
Private Const TOLERANCE As Long = 16

Private Const CLASS_TAG As String = "List: Bedrock"
Private Const ERR_BASE As Long = vbObjectError + 4000
Private Const ERR_INDEX As Long = ERR_BASE + 1
Private Const ERR_EMPTY As Long = ERR_BASE + 2
Private Const ERR_ARG As Long = ERR_BASE + 3
Private Const ERR_OVERFLOW As Long = ERR_BASE + 4
Private Const ERR_UNSUPPORTED As Long = ERR_BASE + 5

Private data As TList

Private Sub Class_Initialize()
    data.Capacity = DEFAULT_CAP
    data.Count = 0
    ReDim data.Items(0 To data.Capacity - 1)
End Sub

Private Sub Class_Terminate()
    Erase data.Items
    data.Capacity = 0
    data.Count = 0
End Sub

' ===================== helpers / validação / util =====================

Private Sub RaiseIndexError(Optional ByVal msg As String = "Index out of bounds")
    Err.Raise ERR_INDEX, CLASS_TAG, msg
End Sub

Private Sub RaiseEmptyError(Optional ByVal msg As String = "List is empty")
    Err.Raise ERR_EMPTY, CLASS_TAG, msg
End Sub

Private Sub RaiseArgError(ByVal msg As String)
    Err.Raise ERR_ARG, CLASS_TAG, msg
End Sub

Private Sub CheckIndex0ToCountMinus1(ByVal Index As Long)
    If Index < 0 Or Index >= data.Count Then RaiseIndexError
End Sub

Private Sub CheckIndex0ToCount(ByVal Index As Long)
    If Index < 0 Or Index > data.Count Then RaiseIndexError
End Sub

Private Sub CopySlot(ByVal src As Long, ByVal dst As Long)
    If IsObject(data.Items(src)) Then
        Set data.Items(dst) = data.Items(src)
    Else
        data.Items(dst) = data.Items(src)
    End If
End Sub

Private Sub WriteSlot(ByVal idx As Long, ByVal value As Variant)
    If IsObject(value) Then
        Set data.Items(idx) = value
    Else
        data.Items(idx) = value
    End If
End Sub

Private Sub ClearSlot(ByVal idx As Long)
    If IsObject(data.Items(idx)) Then
        Set data.Items(idx) = Nothing
    Else
        data.Items(idx) = Empty
    End If
End Sub

Private Sub EnsureCapacity(Optional ByVal additional As Long = 1)
    If additional < 0 Then additional = 0
    Dim required As Long
    required = data.Count + additional
    If required <= data.Capacity Then Exit Sub
    
    Dim newCap As Long
    newCap = data.Capacity
    If newCap < DEFAULT_CAP Then newCap = DEFAULT_CAP
    If newCap = 0 Then newCap = DEFAULT_CAP
    
    Do While newCap < required
        If newCap < GROW_THRESHOLD Then
            If newCap > 1073741823 Then Err.Raise ERR_OVERFLOW, CLASS_TAG, "Requested capacity too large"
            newCap = newCap * 2
        Else
            Dim inc As Long
            inc = CLng(newCap * 0.5)
            If inc < 1 Then inc = 1
            If newCap > (2147483647 - inc) Then Err.Raise ERR_OVERFLOW, CLASS_TAG, "Requested capacity too large"
            newCap = newCap + inc
        End If
    Loop
    
    data.Capacity = newCap
    ReDim Preserve data.Items(0 To data.Capacity - 1)
End Sub

Private Function AreEqual(ByVal a As Variant, ByVal b As Variant) As Boolean
    If IsObject(a) And IsObject(b) Then
        AreEqual = (a Is b)
    ElseIf IsObject(a) Or IsObject(b) Then
        AreEqual = False
    ElseIf IsNull(a) And IsNull(b) Then
        AreEqual = True
    ElseIf IsNull(a) Or IsNull(b) Then
        AreEqual = False
    Else
        On Error GoTo mismatch
        AreEqual = (a = b)
        On Error GoTo 0
        Exit Function
mismatch:
        AreEqual = False
        On Error GoTo 0
    End If
End Function

Private Function CompareValues(ByVal a As Variant, ByVal b As Variant, ByVal textCompare As VbCompareMethod) As Long
    ' -1: a<b, 0: a=b, 1: a>b
    If IsNull(a) And IsNull(b) Then CompareValues = 0: Exit Function
    If IsNull(a) Then CompareValues = -1: Exit Function
    If IsNull(b) Then CompareValues = 1: Exit Function
    
    If VarType(a) = vbString And VarType(b) = vbString Then
        CompareValues = Sgn(StrComp(CStr(a), CStr(b), textCompare))
        Exit Function
    End If
    
    On Error GoTo mismatch
    If a = b Then CompareValues = 0: Exit Function
    If a < b Then CompareValues = -1 Else CompareValues = 1
    Exit Function
mismatch:
    Err.Raise ERR_UNSUPPORTED, CLASS_TAG, "Incompatible types for comparison or list contains objects"
End Function

' ============================== propriedades ==============================

Public Property Get length() As Long
    length = data.Count
End Property

Public Property Get Capacity() As Long
    Capacity = data.Capacity
End Property

Public Property Let Capacity(ByVal newCap As Long)
    If newCap < 0 Then RaiseArgError ("Capacity must be >= 0")
    If newCap < data.Count Then RaiseArgError ("Capacity cannot be less than Count; use Trim to shrink to Count")
    If newCap <> data.Capacity Then
        data.Capacity = IIf(newCap < DEFAULT_CAP, DEFAULT_CAP, newCap)
        ReDim Preserve data.Items(0 To data.Capacity - 1)
    End If
End Property

Public Function IsEmpty() As Boolean
    IsEmpty = (data.Count = 0)
End Function

Public Function At(ByVal Index As Long) As Variant
    CheckIndex0ToCountMinus1 Index
    If IsObject(data.Items(Index)) Then
        Set At = data.Items(Index)
    Else
        At = data.Items(Index)
    End If
End Function

Public Property Get Item(ByVal Index As Long) As Variant
    CheckIndex0ToCountMinus1 Index
    If IsObject(data.Items(Index)) Then
        Set Item = data.Items(Index)
    Else
        Item = data.Items(Index)
    End If
End Property

Public Property Let Item(ByVal Index As Long, ByVal value As Variant)
Attribute Item.VB_UserMemId = 0
    CheckIndex0ToCountMinus1 Index
    WriteSlot Index, value
End Property

Public Property Set Item(ByVal Index As Long, ByVal value As Object)
    CheckIndex0ToCountMinus1 Index
    Set data.Items(Index) = value
End Property

Public Property Get Items() As Variant
    Dim arr() As Variant
    If data.Count = 0 Then
        ReDim arr(-1 To -1)
    Else
        Dim i As Long
        ReDim arr(0 To data.Count - 1)
        For i = 0 To data.Count - 1
            If IsObject(data.Items(i)) Then
                Set arr(i) = data.Items(i)
            Else
                arr(i) = data.Items(i)
            End If
        Next i
    End If
    Items = arr
End Property

Public Property Let Items(ByVal newItems As Variant)
    If Not IsArray(newItems) Then RaiseArgError ("Assigned value must be a 1-D array")
    Dim lb As Long, ub As Long, cnt As Long
    On Error GoTo badDim
    lb = LBound(newItems, 1)
    ub = UBound(newItems, 1)
    cnt = ub - lb + 1
    On Error GoTo 0
    
    data.Count = cnt
    data.Capacity = IIf(cnt < DEFAULT_CAP, DEFAULT_CAP, cnt)
    ReDim data.Items(0 To data.Capacity - 1)
    
    Dim i As Long, v As Variant
    For i = 0 To cnt - 1
        v = newItems(lb + i)
        WriteSlot i, v
    Next i
    Exit Property
badDim:
    RaiseArgError ("Assigned value must be a 1-D array")
End Property

' ============================== operações básicas ==============================

Public Sub Add(ByVal value As Variant)
    EnsureCapacity 1 + TOLERANCE
    WriteSlot data.Count, value
    data.Count = data.Count + 1
End Sub

Public Sub AddRange(ByVal source As Variant)
    Dim arr As Variant, lb As Long, ub As Long, n As Long
    If IsObject(source) Then
        If TypeOf source Is List Then
            arr = source.ToArray
        Else
            RaiseArgError ("AddRange accepts a 1-D array or a List")
        End If
    ElseIf IsArray(source) Then
        arr = source
    Else
        RaiseArgError ("AddRange requires a 1-D array or List")
    End If
    
    On Error GoTo badDim
    lb = LBound(arr, 1): ub = UBound(arr, 1): n = ub - lb + 1
    On Error GoTo 0
    
    If n <= 0 Then Exit Sub
    EnsureCapacity n
    
    Dim base As Long: base = data.Count
    Dim i As Long, v As Variant
    For i = 0 To n - 1
        v = arr(lb + i)
        WriteSlot base + i, v
    Next i
    data.Count = data.Count + n
    Exit Sub
badDim:
    RaiseArgError ("AddRange expects a 1-D array")
End Sub

Public Sub InsertAt(ByVal Index As Long, ByVal value As Variant)
    CheckIndex0ToCount Index
    EnsureCapacity 1 + TOLERANCE
    Dim i As Long
    For i = data.Count To Index + 1 Step -1
        CopySlot i - 1, i
    Next i
    WriteSlot Index, value
    data.Count = data.Count + 1
End Sub

Public Sub InsertRange(ByVal Index As Long, ByVal source As Variant)
    CheckIndex0ToCount Index
    Dim arr As Variant, lb As Long, ub As Long, n As Long
    If IsObject(source) Then
        If TypeOf source Is List Then
            arr = source.ToArray
        Else
            RaiseArgError ("InsertRange accepts a 1-D array or a List")
        End If
    ElseIf IsArray(source) Then
        arr = source
    Else
        RaiseArgError ("InsertRange requires a 1-D array or List")
    End If
    
    On Error GoTo badDim
    lb = LBound(arr, 1): ub = UBound(arr, 1): n = ub - lb + 1
    On Error GoTo 0
    
    If n <= 0 Then Exit Sub
    EnsureCapacity n + TOLERANCE
    
    Dim i As Long, v As Variant
    For i = data.Count - 1 To Index Step -1
        CopySlot i, i + n
    Next i
    
    For i = 0 To n - 1
        v = arr(lb + i)
        WriteSlot Index + i, v
    Next i
    
    data.Count = data.Count + n
    Exit Sub
badDim:
    RaiseArgError ("InsertRange expects a 1-D array")
End Sub

Public Sub RemoveAt(ByVal Index As Long)
    CheckIndex0ToCountMinus1 Index
    Dim i As Long
    For i = Index To data.Count - 2
        CopySlot i + 1, i
    Next i
    ClearSlot data.Count - 1
    data.Count = data.Count - 1
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
    If data.Count = 0 Then RaiseEmptyError
    Dim idx As Long: idx = data.Count - 1
    If IsObject(data.Items(idx)) Then
        Set Pop = data.Items(idx)
        Set data.Items(idx) = Nothing
    Else
        Pop = data.Items(idx)
        data.Items(idx) = Empty
    End If
    data.Count = data.Count - 1
End Function

Public Function Shift() As Variant
    If data.Count = 0 Then RaiseEmptyError
    If IsObject(data.Items(0)) Then
        Set Shift = data.Items(0)
    Else
        Shift = data.Items(0)
    End If
    RemoveAt 0
End Function

Public Sub Clear()
    data.Capacity = DEFAULT_CAP
    data.Count = 0
    ReDim data.Items(0 To data.Capacity - 1)
End Sub

Public Sub Trim()
    If data.Count <= 0 Then
        Clear
    Else
        data.Capacity = data.Count
        ReDim Preserve data.Items(0 To data.Capacity - 1)
    End If
End Sub

Public Sub Reserve(ByVal additionalCapacity As Long)
    If additionalCapacity < 0 Then RaiseArgError ("additionalCapacity must be >= 0")
    EnsureCapacity additionalCapacity
End Sub

' ============================== utilitários ==============================

Public Sub Shuffle()
    Dim i As Long, j As Long
    Dim tmp As Variant
    Randomize
    For i = data.Count - 1 To 1 Step -1
        j = Int((i + 1) * Rnd)
        If IsObject(data.Items(i)) Then
            Set tmp = data.Items(i)
        Else
            tmp = data.Items(i)
        End If
        
        If IsObject(data.Items(j)) Then
            Set data.Items(i) = data.Items(j)
        Else
            data.Items(i) = data.Items(j)
        End If
        
        If IsObject(tmp) Then
            Set data.Items(j) = tmp
        Else
            data.Items(j) = tmp
        End If
    Next i
End Sub

Public Function Join(Optional ByVal delimiter As String = ",") As String
    If data.Count = 0 Then
        Join = ""
        Exit Function
    End If
    Dim s() As String
    ReDim s(0 To data.Count - 1)
    Dim i As Long, v As Variant
    For i = 0 To data.Count - 1
        v = data.Items(i)
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
    j = data.Count - 1
    For i = 0 To (data.Count \ 2) - 1
        Swap i, j
        j = j - 1
    Next i
End Sub

Public Function ToArray() As Variant
    Dim arr() As Variant
    If data.Count <= 0 Then
        ReDim arr(-1 To -1)
    Else
        ReDim arr(0 To data.Count - 1)
        Dim i As Long
        For i = 0 To data.Count - 1
            If IsObject(data.Items(i)) Then
                Set arr(i) = data.Items(i)
            Else
                arr(i) = data.Items(i)
            End If
        Next i
    End If
    ToArray = arr
End Function

' ============================== buscas ==============================

Public Function IndexOf(ByVal value As Variant, Optional ByVal startIndex As Long = 0, Optional ByVal textCompare As VbCompareMethod = vbBinaryCompare) As Long
    If startIndex < 0 Then startIndex = 0
    Dim i As Long
    For i = startIndex To data.Count - 1
        If IsObject(data.Items(i)) Or IsObject(value) Then
            If AreEqual(data.Items(i), value) Then IndexOf = i: Exit Function
        ElseIf VarType(data.Items(i)) = vbString And VarType(value) = vbString Then
            If StrComp(CStr(data.Items(i)), CStr(value), textCompare) = 0 Then IndexOf = i: Exit Function
        Else
            If AreEqual(data.Items(i), value) Then IndexOf = i: Exit Function
        End If
    Next i
    IndexOf = -1
End Function

Public Function LastIndexOf(ByVal value As Variant, Optional ByVal startIndex As Long = -1, Optional ByVal textCompare As VbCompareMethod = vbBinaryCompare) As Long
    Dim i As Long
    If startIndex < 0 Or startIndex >= data.Count Then startIndex = data.Count - 1
    For i = startIndex To 0 Step -1
        If IsObject(data.Items(i)) Or IsObject(value) Then
            If AreEqual(data.Items(i), value) Then LastIndexOf = i: Exit Function
        ElseIf VarType(data.Items(i)) = vbString And VarType(value) = vbString Then
            If StrComp(CStr(data.Items(i)), CStr(value), textCompare) = 0 Then LastIndexOf = i: Exit Function
        Else
            If AreEqual(data.Items(i), value) Then LastIndexOf = i: Exit Function
        End If
    Next i
    LastIndexOf = -1
End Function

Public Function Contains(ByVal value As Variant, Optional ByVal textCompare As VbCompareMethod = vbBinaryCompare) As Boolean
    Contains = (IndexOf(value, 0, textCompare) <> -1)
End Function

Public Function Includes(ByVal value As Variant, Optional ByVal textCompare As VbCompareMethod = vbBinaryCompare) As Boolean
    Includes = (IndexOf(value, 0, textCompare) <> -1)
End Function

Public Function BinarySearch(ByVal value As Variant, Optional ByVal ascending As Boolean = True, Optional ByVal textCompare As VbCompareMethod = vbBinaryCompare, Optional ByVal returnInsertionPoint As Boolean = False) As Long
    If data.Count = 0 Then
        BinarySearch = IIf(returnInsertionPoint, -1, -1)
        Exit Function
    End If
    
    Dim i As Long
    For i = 0 To data.Count - 1
        If IsObject(data.Items(i)) Then Err.Raise ERR_UNSUPPORTED, CLASS_TAG, "BinarySearch does not support objects"
    Next i
    
    Dim low As Long, high As Long, mid As Long, cmp As Long
    low = 0: high = data.Count - 1
    
    Do While low <= high
        mid = (low + high) \ 2
        cmp = CompareValues(data.Items(mid), value, textCompare)
        If cmp = 0 Then
            BinarySearch = mid
            Exit Function
        End If
        
        If ascending Then
            If cmp < 0 Then
                low = mid + 1
            Else
                high = mid - 1
            End If
        Else
            If cmp > 0 Then
                low = mid + 1
            Else
                high = mid - 1
            End If
        End If
    Loop
    
    If returnInsertionPoint Then
        BinarySearch = -(low + 1)
    Else
        BinarySearch = -1
    End If
End Function

' ============================== higher-order ==============================

Public Sub ForEach(ByVal callback As Object)
    Dim i As Long
    For i = 0 To data.Count - 1
        CallByName callback, "Invoke", VbMethod, data.Items(i)
    Next i
End Sub

Public Function Every(ByVal callback As Object) As Boolean
    Dim i As Long
    For i = 0 To data.Count - 1
        If Not CallByName(callback, "Invoke", VbMethod, data.Items(i)) Then
            Every = False: Exit Function
        End If
    Next i
    Every = True
End Function

Public Function Some(ByVal callback As Object) As Boolean
    Dim i As Long
    For i = 0 To data.Count - 1
        If CallByName(callback, "Invoke", VbMethod, data.Items(i)) Then
            Some = True: Exit Function
        End If
    Next i
    Some = False
End Function

Public Function Map(ByVal callback As Object) As List
    Dim result As New List, i As Long, mapped As Variant
    For i = 0 To data.Count - 1
        mapped = CallByName(callback, "Invoke", VbMethod, data.Items(i))
        result.Add mapped
    Next i
    Set Map = result
End Function

Public Function Filter(ByVal callback As Object) As List
    Dim result As New List, i As Long, keep As Boolean
    For i = 0 To data.Count - 1
        keep = CallByName(callback, "Invoke", VbMethod, data.Items(i))
        If keep Then result.Add data.Items(i)
    Next i
    Set Filter = result
End Function

Public Function Reduce(ByVal callback As Object, ByVal initial As Variant) As Variant
    Dim acc As Variant: acc = initial
    Dim i As Long
    For i = 0 To data.Count - 1
        acc = CallByName(callback, "Invoke", VbMethod, acc, data.Items(i))
    Next i
    Reduce = acc
End Function

Public Function FindAll(ByVal callback As Object) As List
    Set FindAll = Filter(callback)
End Function

Public Function Slice(ByVal startIndex As Long, ByVal endIndex As Long) As List
    Dim result As New List, i As Long
    If startIndex < 0 Then startIndex = 0
    If endIndex > data.Count Then endIndex = data.Count
    If endIndex < startIndex Then
        Set Slice = result
        Exit Function
    End If
    For i = startIndex To endIndex - 1
        result.Add data.Items(i)
    Next i
    Set Slice = result
End Function

Public Function Concat(ByVal other As Variant) As List
    Dim result As New List
    result.AddRange Me.ToArray
    
    If IsObject(other) Then
        If TypeOf other Is List Then
            result.AddRange other.ToArray
        Else
            RaiseArgError ("Concat accepts a List or a 1-D array")
        End If
    ElseIf IsArray(other) Then
        result.AddRange other
    Else
        RaiseArgError ("Concat accepts a List or a 1-D array")
    End If
    
    Set Concat = result
End Function

Public Function Clone() As List
    Dim result As New List
    result.AddRange Me.ToArray
    Set Clone = result
End Function

' ============================== ordenação ==============================

Private Sub Swap(ByVal i As Long, ByVal j As Long)
    If i = j Then Exit Sub
    Dim tmp As Variant
    If IsObject(data.Items(i)) Then
        Set tmp = data.Items(i)
    Else
        tmp = data.Items(i)
    End If
    
    If IsObject(data.Items(j)) Then
        Set data.Items(i) = data.Items(j)
    Else
        data.Items(i) = data.Items(j)
    End If
    
    If IsObject(tmp) Then
        Set data.Items(j) = tmp
    Else
        data.Items(j) = tmp
    End If
End Sub

Private Sub InsertionSort(ByVal low As Long, ByVal high As Long, ByVal asc As Boolean, ByVal textCompare As VbCompareMethod)
    Dim i As Long, j As Long, key As Variant
    For i = low + 1 To high
        key = data.Items(i)
        j = i - 1
        If asc Then
            Do While j >= low And CompareValues(data.Items(j), key, textCompare) > 0
                data.Items(j + 1) = data.Items(j)
                j = j - 1
            Loop
        Else
            Do While j >= low And CompareValues(data.Items(j), key, textCompare) < 0
                data.Items(j + 1) = data.Items(j)
                j = j - 1
            Loop
        End If
        data.Items(j + 1) = key
    Next i
End Sub

Public Sub Sort(Optional ByVal ascending As Boolean = True, Optional ByVal textCompare As VbCompareMethod = vbBinaryCompare)
    If data.Count < 2 Then Exit Sub
    Dim i As Long
    For i = 0 To data.Count - 1
        If IsObject(data.Items(i)) Then Err.Raise ERR_UNSUPPORTED, CLASS_TAG, "Sort does not support objects"
    Next i
    QuickSort 0, data.Count - 1, ascending, 0, textCompare
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
    i = low: j = high: pivot = data.Items((low + high) \ 2)
    
    Do While i <= j
        If asc Then
            Do While CompareValues(data.Items(i), pivot, textCompare) < 0: i = i + 1: Loop
            Do While CompareValues(data.Items(j), pivot, textCompare) > 0: j = j - 1: Loop
        Else
            Do While CompareValues(data.Items(i), pivot, textCompare) > 0: i = i + 1: Loop
            Do While CompareValues(data.Items(j), pivot, textCompare) < 0: j = j - 1: Loop
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
