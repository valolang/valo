VERSION 1.0 CLASS
BEGIN
  MultiUse = -1  'True
END
Attribute VB_Name = "ArrayZ"
Attribute VB_GlobalNameSpace = False
Attribute VB_Creatable = False
Attribute VB_PredeclaredId = False
Attribute VB_Exposed = False
' ***************************************************************************
' ArrayZ - High-performance Dynamic Array for VBA
' ***************************************************************************
' This class module implements a dynamic array-like structure with fast
' resizing, insertion, removal, and search functionalities. It uses pure VBA
' and optimizes performance by managing capacity doubling and minimal memory
' reallocations.
'
' Key Features:
' - Dynamic capacity management (doubling strategy)
' - Indexed access with bounds checking
' - Add, InsertAt, RemoveAt methods for flexible array operations
' - Search methods: IndexOf, LastIndexOf, Contains
' - Utilities: Shuffle, Reverse, Clear, Trim, ToArray
' ---------------------------------------------------------------------------
' References
' ---------------------------------------------------------------------------
' - VBA Dynamic Arrays & ReDim Preserve: https://docs.microsoft.com/en-us/office/vba/language/reference/user-interface-help/redim-statement
' - VBA Variant & Object Handling: https://docs.microsoft.com/en-us/office/vba/language/reference/user-interface-help/variant-data-type
' - Fisher-Yates Shuffle Algorithm: https://en.wikipedia.org/wiki/Fisher%E2%80%93Yates_shuffle
'
' ---------------------------------------------------------------------------
' Team: Bedrock Team
' Last Update: 15/05/2025
' Version: 1.0
' ***************************************************************************


Option Explicit

Private Type TArrayZ
    Items() As Variant
    Count As Long
    Capacity As Long
End Type

Private data As TArrayZ

Private Sub Class_Initialize()
    data.Capacity = 8
    data.Count = 0
    ReDim data.Items(0 To data.Capacity - 1)
End Sub

Private Sub EnsureCapacity()
    If data.Count >= data.Capacity Then
        data.Capacity = IIf(data.Capacity < 1024, data.Capacity * 2, data.Capacity * 1.5)
        ReDim Preserve data.Items(0 To data.Capacity - 1)
    End If
End Sub

Public Property Get Length() As Long
    Length = data.Count
End Property

Public Property Get Capacity() As Long
    Capacity = data.Capacity
End Property

Public Property Get Item(Index As Long) As Variant
    If Index < 0 Or Index >= data.Count Then Err.Raise vbObjectError + 1, "ArrayZ", "Index out of bounds"
    Item = data.Items(Index)
End Property

Public Property Let Item(Index As Long, ByVal Value As Variant)
Attribute Item.VB_UserMemId = 0
    If Index < 0 Or Index >= data.Count Then Err.Raise vbObjectError + 1, "ArrayZ", "Index out of bounds"
    data.Items(Index) = Value
End Property

Public Property Get Items() As Variant
    Dim Arr() As Variant
    If data.Count > 0 Then
        ReDim Arr(0 To data.Count - 1)
        Dim i As Long
        For i = 0 To data.Count - 1
            Arr(i) = data.Items(i)
        Next i
    Else
        ReDim Arr(-1 To -1)
    End If
    Items = Arr
End Property

Public Property Let Items(ByVal newItems As Variant)
    If Not IsArray(newItems) Then
        Err.Raise vbObjectError + 2, "ArrayZ", "Assigned value must be an array"
    End If

    data.Count = UBound(newItems) - LBound(newItems) + 1
    data.Capacity = data.Count

    If data.Capacity < 8 Then data.Capacity = 8

    ReDim data.Items(0 To data.Capacity - 1)

    Dim i As Long
    For i = 0 To data.Count - 1
        data.Items(i) = newItems(LBound(newItems) + i)
    Next i
End Property

Public Sub Add(ByVal Value As Variant)
    EnsureCapacity
    
    If TypeOf Value Is Object  Then
        Set data.Items(data.Count) = Value
    Else
        data.Items(data.Count) = Value
    End If
    data.Count = data.Count + 1
End Sub

Public Sub InsertAt(Index As Long, ByVal Value As Variant)
    If Index < 0 Or Index > data.Count Then Err.Raise vbObjectError + 1, "ArrayZ", "Index out of bounds"
    EnsureCapacity
    Dim i As Long
    For i = data.Count To Index + 1 Step -1
        data.Items(i) = data.Items(i - 1)
    Next i
    data.Items(Index) = Value
    data.Count = data.Count + 1
End Sub

Public Sub RemoveAt(Index As Long)
    If Index < 0 Or Index >= data.Count Then Err.Raise vbObjectError + 1, "ArrayZ", "Index out of bounds"
    Dim i As Long
    For i = Index To data.Count - 2
        data.Items(i) = data.Items(i + 1)
    Next i
    data.Count = data.Count - 1
    data.Items(data.Count) = Empty
End Sub

Public Sub Clear()
    data.Capacity = 8
    data.Count = 0
    ReDim data.Items(0 To data.Capacity - 1)
End Sub

Public Sub Trim()
    If data.Count > 0 Then
        data.Capacity = data.Count
        ReDim Preserve data.Items(0 To data.Capacity - 1)
    Else
        Clear
    End If
End Sub

Public Sub Shuffle()
    Dim i As Long
    Dim j As Long
    Dim temp As Variant
    
    Randomize
    
    For i = data.Count - 1 To 1 Step -1
        j = Int((i + 1) * Rnd)
        temp = data.Items(i)
        data.Items(i) = data.Items(j)
        data.Items(j) = temp
    Next i
End Sub

Public Sub Reverse()
    Dim i As Long, j As Long
    Dim temp As Variant
    j = data.Count - 1
    For i = 0 To (data.Count \ 2) - 1
        temp = data.Items(i)
        data.Items(i) = data.Items(j - i)
        data.Items(j - i) = temp
    Next i
End Sub

Public Function ToArray() As Variant
    Dim Arr() As Variant
    If data.Count > 0 Then
        ReDim Arr(0 To data.Count - 1)
        Dim i As Long
        For i = 0 To data.Count - 1
            Arr(i) = data.Items(i)
        Next i
    Else
        ReDim Arr(-1 To -1)
    End If
    ToArray = Arr
End Function

Public Function IndexOf(ByVal Value As Variant) As Long
    Dim i As Long
    For i = 0 To data.Count - 1
        If IsObject(data.Items(i)) And IsObject(Value) Then
            If data.Items(i) Is Value Then
                IndexOf = i
                Exit Function
            End If
        Else
            If data.Items(i) = Value Then
                IndexOf = i
                Exit Function
            End If
        End If
    Next i
    IndexOf = -1
End Function

Public Function Contains(ByVal Value As Variant) As Boolean
    Contains = (IndexOf(Value) <> -1)
End Function

Public Function LastIndexOf(ByVal Value As Variant) As Long
    Dim i As Long
    For i = data.Count - 1 To 0 Step -1
        If IsObject(data.Items(i)) And IsObject(Value) Then
            If data.Items(i) Is Value Then
                LastIndexOf = i
                Exit Function
            End If
        Else
            If data.Items(i) = Value Then
                LastIndexOf = i
                Exit Function
            End If
        End If
    Next i
    LastIndexOf = -1
End Function
