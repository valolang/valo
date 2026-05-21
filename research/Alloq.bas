Attribute VB_Name = "Alloq"
' ============================================================================
' Alloq v1.0.0
' Copyright (c) 2026 Bedrock
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

#If Mac Then
    #If VBA7 Then
        Private Declare PtrSafe Function CopyMemory Lib "/usr/lib/libc.dylib" Alias "memmove" (Destination As Any, Source As Any, ByVal Length As LongPtr) As LongPtr
        Private Declare PtrSafe Function FillMemory Lib "/usr/lib/libc.dylib" Alias "memset" (Destination As Any, ByVal Fill As Byte, ByVal Length As LongPtr) As LongPtr
        Public Declare PtrSafe Function MemCopy Lib "/usr/lib/libc.dylib" Alias "memmove" (ByVal Destination As LongPtr, ByVal Source As LongPtr, ByVal Length As LongPtr) As LongPtr
    #Else
        Private Declare Function CopyMemory Lib "/usr/lib/libc.dylib" Alias "memmove" (Destination As Any, Source As Any, ByVal Length As Long) As Long
        Private Declare Function FillMemory Lib "/usr/lib/libc.dylib" Alias "memset" (Destination As Any, ByVal Fill As Byte, ByVal Length As Long) As Long
        Public Declare Function MemCopy Lib "/usr/lib/libc.dylib" Alias "memmove" (ByVal Destination As Long, ByVal Source As Long, ByVal Length As Long) As Long
    #End If
#Else
    #If VBA7 Then
        Private Declare PtrSafe Sub CopyMemory Lib "kernel32" Alias "RtlMoveMemory" (Destination As Any, Source As Any, ByVal Length As LongPtr)
        Private Declare PtrSafe Sub FillMemory Lib "kernel32" Alias "RtlFillMemory" (Destination As Any, ByVal Length As LongPtr, ByVal Fill As Byte)
        #If TWINBASIC Then
            Public Declare PtrSafe Sub MemCopy Lib "kernel32" Alias "RtlMoveMemory" (ByVal Destination As LongPtr, ByVal Source As LongPtr, ByVal Length As LongPtr)
        #End If
    #Else
        Private Declare Sub CopyMemory Lib "kernel32" Alias "RtlMoveMemory" (Destination As Any, Source As Any, ByVal Length As Long)
        Private Declare Sub FillMemory Lib "kernel32" Alias "RtlFillMemory" (Destination As Any, ByVal Length As Long, ByVal Fill As Byte)
        Public Declare Sub MemCopy Lib "kernel32" Alias "RtlMoveMemory" (ByVal Destination As Long, ByVal Source As Long, ByVal Length As Long)
        Public Declare Sub MemFill Lib "kernel32" Alias "RtlFillMemory" (ByVal Destination As Long, ByVal Length As Long, ByVal Fill As Byte)
    #End If
#End If

#If VBA7 = 0 Then
    Public Enum LongPtr
        [_]
    End Enum
#End If

#If Win64 Then
    Public Const PTR_SIZE As Long = 8
    Public Const VARIANT_SIZE As Long = 24
    Public Const NULL_PTR As LongLong = 0^
#Else
    Public Const PTR_SIZE As Long = 4
    Public Const VARIANT_SIZE As Long = 16
    Public Const NULL_PTR As Long = 0&
#End If

Private Const BYTE_SIZE As Long = 1
Private Const INT_SIZE As Long = 2
Private Const LONG_SIZE As Long = 4

#If Win64 Then
    #If Mac Then
        Public Const vbLongLong As Long = 20
    #End If
    Public Const vbLongPtr As Long = vbLongLong
#Else
    Public Const vbLongLong As Long = 20
    Public Const vbLongPtr As Long = vbLong
#End If

Public Const VT_BYREF As Long = &H4000

Public Type SAFEARRAYBOUND
    cElements As Long
    lLbound As Long
End Type

Public Type SAFEARRAY_1D
    cDims As Integer
    fFeatures As Integer
    cbElements As Long
    cLocks As Long
    pvData As LongPtr
    rgsabound0 As SAFEARRAYBOUND
End Type

Public Enum SAFEARRAY_FEATURES
    FADF_AUTO = &H1
    FADF_STATIC = &H2
    FADF_EMBEDDED = &H4
    FADF_FIXEDSIZE = &H10
    FADF_RECORD = &H20
    FADF_HAVEIID = &H40
    FADF_HAVEVARTYPE = &H80
    FADF_BSTR = &H100
    FADF_UNKNOWN = &H200
    FADF_DISPATCH = &H400
    FADF_VARIANT = &H800
    FADF_RESERVED = &HF008
End Enum

Public Enum SAFEARRAY_OFFSETS
    cDimsOffset = 0
    fFeaturesOffset = cDimsOffset + INT_SIZE
    cbElementsOffset = fFeaturesOffset + INT_SIZE
    cLocksOffset = cbElementsOffset + LONG_SIZE
    pvDataOffset = cLocksOffset + PTR_SIZE
    rgsaboundOffset = pvDataOffset + PTR_SIZE
    rgsabound0_cElementsOffset = rgsaboundOffset
    rgsabound0_lLboundOffset = rgsabound0_cElementsOffset + LONG_SIZE
End Enum

Public Const SAFEARRAY_SIZE As Long = rgsabound0_lLboundOffset + LONG_SIZE

Private Type Byte8:   l As Long:     r As Long:     End Type
Private Type Byte16:  l As Byte8:    r As Byte8:    End Type
Private Type Byte32:  l As Byte16:   r As Byte16:   End Type
Private Type Byte64:  l As Byte32:   r As Byte32:   End Type
Private Type Byte128: l As Byte64:   r As Byte64:   End Type
Private Type Byte256: l As Byte128:  r As Byte128:  End Type
Private Type Byte512: l As Byte256:  r As Byte256:  End Type
Private Type Byte1K:  l As Byte512:  r As Byte512:  End Type
Private Type Byte2K:  l As Byte1K:   r As Byte1K:   End Type
Private Type Byte4K:  l As Byte2K:   r As Byte2K:   End Type
Private Type Byte8K:  l As Byte4K:   r As Byte4K:   End Type
Private Type Byte16K: l As Byte8K:   r As Byte8K:   End Type
Private Type Byte32K: l As Byte16K:  r As Byte16K:  End Type

Private Type ArrayAccessor
    dPtr() As LongPtr:  dByte() As Byte:  dBool() As Boolean
    dInt() As Integer:  dLong() As Long:  dSng()  As Single
    dCur() As Currency: dDate() As Date:  dDbl()  As Double
    dVar() As Variant:  dObj() As Object: dStr() As String
#If Win64 Then
    dLongLong() As LongLong
#End If
    b16()  As Byte16:  b32()  As Byte32:  b64()  As Byte64
    b128() As Byte128: b256() As Byte256: b512() As Byte512
    b1K()  As Byte1K:  b2K()  As Byte2K:  b4K()  As Byte4K
    b8K()  As Byte8K:  b16K() As Byte16K: b32K() As Byte32K
    s16()  As String * 8:    s32()  As String * 16:   s64()  As String * 32
    s128() As String * 64:   s256() As String * 128:  s512() As String * 256
    s1K()  As String * 512:  s2K()  As String * 1024: s4K()  As String * 2048
    s8K()  As String * 4096: s16K() As String * 8192: s32K() As String * 16384
    sa() As SAFEARRAY_1D
End Type

Private Type ByteInfo
    bit(0 To 7) As Boolean
End Type

Public Type MEMORY_ACCESSOR
    isSet As Boolean
    ac As ArrayAccessor
    sa As SAFEARRAY_1D
End Type

Private m_allocMemory As New Collection

Public Sub InitMemoryAccessor(ByRef maToInit As MEMORY_ACCESSOR)
    If maToInit.isSet Then Exit Sub

    Static ma As MEMORY_ACCESSOR
    Dim saPtr As LongPtr: saPtr = VarPtr(maToInit.sa)
    Dim i As Long

    If Not ma.isSet Then
        With ma.sa
            .cDims = 1
            .cbElements = PTR_SIZE
            .cLocks = 1
            .fFeatures = FADF_AUTO Or FADF_FIXEDSIZE
        End With
        CopyMemory ByVal VarPtr(ma.ac), VarPtr(ma.sa), PTR_SIZE
        ma.isSet = True
    End If

    With maToInit.sa
        .cDims = 1
        .cLocks = 1
        .fFeatures = FADF_AUTO Or FADF_FIXEDSIZE
    End With

    ma.sa.pvData = VarPtr(maToInit.ac)
    ma.sa.rgsabound0.cElements = LenB(maToInit.ac) / PTR_SIZE

    For i = 0 To ma.sa.rgsabound0.cElements - 1
        ma.ac.dPtr(i) = saPtr
    Next i

    ma.sa.rgsabound0.cElements = 0
    ma.sa.pvData = NULL_PTR

    maToInit.isSet = True
End Sub

Public Property Get MemByte(ByVal memAddress As LongPtr) As Byte
    #If Mac Or (VBA7 = 0) Then
        CopyMemory MemByte, ByVal memAddress, 1
    #ElseIf TWINBASIC Then
        GetMem1 memAddress, MemByte
    #Else
        Static ma As MEMORY_ACCESSOR: If Not ma.isSet Then InitMemoryAccessor ma
        ma.sa.pvData = memAddress: ma.sa.rgsabound0.cElements = 1
        MemByte = ma.ac.dByte(0)
        ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR
    #End If
End Property

Public Property Let MemByte(ByVal memAddress As LongPtr, ByVal newValue As Byte)
    #If Mac Or (VBA7 = 0) Then
        CopyMemory ByVal memAddress, newValue, 1
    #ElseIf TWINBASIC Then
        PutMem1 memAddress, newValue
    #Else
        Static ma As MEMORY_ACCESSOR: If Not ma.isSet Then InitMemoryAccessor ma
        ma.sa.pvData = memAddress: ma.sa.rgsabound0.cElements = 1
        ma.ac.dByte(0) = newValue
        ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR
    #End If
End Property

Public Property Get MemInt(ByVal memAddress As LongPtr) As Integer
    #If Mac Or (VBA7 = 0) Then
        CopyMemory MemInt, ByVal memAddress, 2
    #ElseIf TWINBASIC Then
        GetMem2 memAddress, MemInt
    #Else
        Static ma As MEMORY_ACCESSOR: If Not ma.isSet Then InitMemoryAccessor ma
        ma.sa.pvData = memAddress: ma.sa.rgsabound0.cElements = 1
        MemInt = ma.ac.dInt(0)
        ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR
    #End If
End Property

Public Property Let MemInt(ByVal memAddress As LongPtr, ByVal newValue As Integer)
    #If Mac Or (VBA7 = 0) Then
        CopyMemory ByVal memAddress, newValue, 2
    #ElseIf TWINBASIC Then
        PutMem2 memAddress, newValue
    #Else
        Static ma As MEMORY_ACCESSOR: If Not ma.isSet Then InitMemoryAccessor ma
        ma.sa.pvData = memAddress: ma.sa.rgsabound0.cElements = 1
        ma.ac.dInt(0) = newValue
        ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR
    #End If
End Property

Public Property Get MemBool(ByVal memAddress As LongPtr) As Boolean
    #If Mac Or TWINBASIC Or (VBA7 = 0) Then
        CopyMemory MemBool, ByVal memAddress, 2
    #Else
        Static ma As MEMORY_ACCESSOR: If Not ma.isSet Then InitMemoryAccessor ma
        ma.sa.pvData = memAddress: ma.sa.rgsabound0.cElements = 1
        MemBool = ma.ac.dBool(0)
        ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR
    #End If
End Property

Public Property Let MemBool(ByVal memAddress As LongPtr, ByVal newValue As Boolean)
    #If Mac Or TWINBASIC Or (VBA7 = 0) Then
        CopyMemory ByVal memAddress, newValue, 2
    #Else
        Static ma As MEMORY_ACCESSOR: If Not ma.isSet Then InitMemoryAccessor ma
        ma.sa.pvData = memAddress: ma.sa.rgsabound0.cElements = 1
        ma.ac.dBool(0) = newValue
        ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR
    #End If
End Property

Public Property Get MemLong(ByVal memAddress As LongPtr) As Long
    #If Mac Or (VBA7 = 0) Then
        CopyMemory MemLong, ByVal memAddress, 4
    #ElseIf TWINBASIC Then
        GetMem4 memAddress, MemLong
    #Else
        Static ma As MEMORY_ACCESSOR: If Not ma.isSet Then InitMemoryAccessor ma
        ma.sa.pvData = memAddress: ma.sa.rgsabound0.cElements = 1
        MemLong = ma.ac.dLong(0)
        ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR
    #End If
End Property

Public Property Let MemLong(ByVal memAddress As LongPtr, ByVal newValue As Long)
    #If Mac Or (VBA7 = 0) Then
        CopyMemory ByVal memAddress, newValue, 4
    #ElseIf TWINBASIC Then
        PutMem4 memAddress, newValue
    #Else
        Static ma As MEMORY_ACCESSOR: If Not ma.isSet Then InitMemoryAccessor ma
        ma.sa.pvData = memAddress: ma.sa.rgsabound0.cElements = 1
        ma.ac.dLong(0) = newValue
        ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR
    #End If
End Property

Public Property Get MemSng(ByVal memAddress As LongPtr) As Single
    #If Mac Or TWINBASIC Or (VBA7 = 0) Then
        CopyMemory MemSng, ByVal memAddress, 4
    #Else
        Static ma As MEMORY_ACCESSOR: If Not ma.isSet Then InitMemoryAccessor ma
        ma.sa.pvData = memAddress: ma.sa.rgsabound0.cElements = 1
        MemSng = ma.ac.dSng(0)
        ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR
    #End If
End Property

Public Property Let MemSng(ByVal memAddress As LongPtr, ByVal newValue As Single)
    #If Mac Or TWINBASIC Or (VBA7 = 0) Then
        CopyMemory ByVal memAddress, newValue, 4
    #Else
        Static ma As MEMORY_ACCESSOR: If Not ma.isSet Then InitMemoryAccessor ma
        ma.sa.pvData = memAddress: ma.sa.rgsabound0.cElements = 1
        ma.ac.dSng(0) = newValue
        ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR
    #End If
End Property

#If Win64 Or TWINBASIC Then
Public Property Get MemLongLong(ByVal memAddress As LongLong) As LongLong
    #If Mac Or TWINBASIC Or (VBA7 = 0) Then
        CopyMemory MemLongLong, ByVal memAddress, 8
    #Else
        Static ma As MEMORY_ACCESSOR: If Not ma.isSet Then InitMemoryAccessor ma
        ma.sa.pvData = memAddress: ma.sa.rgsabound0.cElements = 1
        MemLongLong = ma.ac.dLongLong(0)
        ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR
    #End If
End Property

Public Property Let MemLongLong(ByVal memAddress As LongLong, ByVal newValue As LongLong)
    #If Mac Or TWINBASIC Or (VBA7 = 0) Then
        CopyMemory ByVal memAddress, newValue, 8
    #Else
        Static ma As MEMORY_ACCESSOR: If Not ma.isSet Then InitMemoryAccessor ma
        ma.sa.pvData = memAddress: ma.sa.rgsabound0.cElements = 1
        ma.ac.dLongLong(0) = newValue
        ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR
    #End If
End Property
#End If

Public Property Get MemLongPtr(ByVal memAddress As LongPtr) As LongPtr
    #If Mac Or (VBA7 = 0) Then
        CopyMemory MemLongPtr, ByVal memAddress, PTR_SIZE
    #ElseIf TWINBASIC Then
        GetMemPtr memAddress, MemLongPtr
    #Else
        Static ma As MEMORY_ACCESSOR: If Not ma.isSet Then InitMemoryAccessor ma
        ma.sa.pvData = memAddress: ma.sa.rgsabound0.cElements = 1
        MemLongPtr = ma.ac.dPtr(0)
        ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR
    #End If
End Property

Public Property Let MemLongPtr(ByVal memAddress As LongPtr, ByVal newValue As LongPtr)
    #If Mac Or (VBA7 = 0) Then
        CopyMemory ByVal memAddress, newValue, PTR_SIZE
    #ElseIf TWINBASIC Then
        PutMemPtr memAddress, newValue
    #Else
        Static ma As MEMORY_ACCESSOR: If Not ma.isSet Then InitMemoryAccessor ma
        ma.sa.pvData = memAddress: ma.sa.rgsabound0.cElements = 1
        ma.ac.dPtr(0) = newValue
        ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR
    #End If
End Property

Public Property Get MemCur(ByVal memAddress As LongPtr) As Currency
    #If Mac Or (VBA7 = 0) Then
        CopyMemory MemCur, ByVal memAddress, 8
    #ElseIf TWINBASIC Then
        GetMem8 memAddress, MemCur
    #Else
        Static ma As MEMORY_ACCESSOR: If Not ma.isSet Then InitMemoryAccessor ma
        ma.sa.pvData = memAddress: ma.sa.rgsabound0.cElements = 1
        MemCur = ma.ac.dCur(0)
        ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR
    #End If
End Property

Public Property Let MemCur(ByVal memAddress As LongPtr, ByVal newValue As Currency)
    #If Mac Or (VBA7 = 0) Then
        CopyMemory ByVal memAddress, newValue, 8
    #ElseIf TWINBASIC Then
        PutMem8 memAddress, newValue
    #Else
        Static ma As MEMORY_ACCESSOR: If Not ma.isSet Then InitMemoryAccessor ma
        ma.sa.pvData = memAddress: ma.sa.rgsabound0.cElements = 1
        ma.ac.dCur(0) = newValue
        ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR
    #End If
End Property

Public Property Get MemDate(ByVal memAddress As LongPtr) As Date
    #If Mac Or TWINBASIC Or (VBA7 = 0) Then
        CopyMemory MemDate, ByVal memAddress, 8
    #Else
        Static ma As MEMORY_ACCESSOR: If Not ma.isSet Then InitMemoryAccessor ma
        ma.sa.pvData = memAddress: ma.sa.rgsabound0.cElements = 1
        MemDate = ma.ac.dDate(0)
        ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR
    #End If
End Property

Public Property Let MemDate(ByVal memAddress As LongPtr, ByVal newValue As Date)
    #If Mac Or TWINBASIC Or (VBA7 = 0) Then
        CopyMemory ByVal memAddress, newValue, 8
    #Else
        Static ma As MEMORY_ACCESSOR: If Not ma.isSet Then InitMemoryAccessor ma
        ma.sa.pvData = memAddress: ma.sa.rgsabound0.cElements = 1
        ma.ac.dDate(0) = newValue
        ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR
    #End If
End Property

Public Property Get MemDbl(ByVal memAddress As LongPtr) As Double
    #If Mac Or TWINBASIC Or (VBA7 = 0) Then
        CopyMemory MemDbl, ByVal memAddress, 8
    #Else
        Static ma As MEMORY_ACCESSOR: If Not ma.isSet Then InitMemoryAccessor ma
        ma.sa.pvData = memAddress: ma.sa.rgsabound0.cElements = 1
        MemDbl = ma.ac.dDbl(0)
        ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR
    #End If
End Property

Public Property Let MemDbl(ByVal memAddress As LongPtr, ByVal newValue As Double)
    #If Mac Or TWINBASIC Or (VBA7 = 0) Then
        CopyMemory ByVal memAddress, newValue, 8
    #Else
        Static ma As MEMORY_ACCESSOR: If Not ma.isSet Then InitMemoryAccessor ma
        ma.sa.pvData = memAddress: ma.sa.rgsabound0.cElements = 1
        ma.ac.dDbl(0) = newValue
        ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR
    #End If
End Property

Public Function MemCompare(ByVal ptr1 As LongPtr, ByVal ptr2 As LongPtr, ByVal bytesCount As Long) As Boolean
    Static ma1 As MEMORY_ACCESSOR: If Not ma1.isSet Then InitMemoryAccessor ma1
    Static ma2 As MEMORY_ACCESSOR: If Not ma2.isSet Then InitMemoryAccessor ma2
    Dim i As Long

    If bytesCount <= 0 Then
        MemCompare = True
        Exit Function
    End If

    ma1.sa.pvData = ptr1
    ma2.sa.pvData = ptr2
    ma1.sa.rgsabound0.cElements = 1
    ma2.sa.rgsabound0.cElements = 1

    Dim remaining As Long: remaining = bytesCount

    Do While remaining >= 8
        ma1.sa.cbElements = 8
        ma2.sa.cbElements = 8
        If ma1.ac.dCur(0) <> ma2.ac.dCur(0) Then GoTo NotEqual
        ma1.sa.pvData = ma1.sa.pvData + 8
        ma2.sa.pvData = ma2.sa.pvData + 8
        remaining = remaining - 8
    Loop

    Do While remaining >= 4
        ma1.sa.cbElements = 4
        ma2.sa.cbElements = 4
        If ma1.ac.dLong(0) <> ma2.ac.dLong(0) Then GoTo NotEqual
        ma1.sa.pvData = ma1.sa.pvData + 4
        ma2.sa.pvData = ma2.sa.pvData + 4
        remaining = remaining - 4
    Loop

    ma1.sa.cbElements = 1
    ma2.sa.cbElements = 1
    Do While remaining > 0
        If ma1.ac.dByte(0) <> ma2.ac.dByte(0) Then GoTo NotEqual
        ma1.sa.pvData = ma1.sa.pvData + 1
        ma2.sa.pvData = ma2.sa.pvData + 1
        remaining = remaining - 1
    Loop

    MemCompare = True
    GoTo Cleanup

NotEqual:
    MemCompare = False

Cleanup:
    ma1.sa.rgsabound0.cElements = 0: ma1.sa.pvData = NULL_PTR
    ma2.sa.rgsabound0.cElements = 0: ma2.sa.pvData = NULL_PTR
End Function

Public Function MemSearch(ByVal startPtr As LongPtr, ByVal searchByte As Byte, ByVal maxBytes As Long) As LongPtr
    Static ma As MEMORY_ACCESSOR: If Not ma.isSet Then InitMemoryAccessor ma
    Dim i As Long

    If maxBytes <= 0 Then
        MemSearch = NULL_PTR
        Exit Function
    End If

    ma.sa.pvData = startPtr
    ma.sa.cbElements = 1
    ma.sa.rgsabound0.cElements = maxBytes

    For i = 0 To maxBytes - 1
        If ma.ac.dByte(i) = searchByte Then
            MemSearch = startPtr + i
            GoTo Cleanup
        End If
    Next i

    MemSearch = NULL_PTR

Cleanup:
    ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR
End Function

Public Function MemSearchPattern(ByVal startPtr As LongPtr, ByRef pattern() As Byte, ByVal maxBytes As Long) As LongPtr
    Static ma As MEMORY_ACCESSOR: If Not ma.isSet Then InitMemoryAccessor ma
    Dim i As Long
    Dim j As Long
    Dim patternLen As Long
    Dim found As Boolean

    patternLen = UBound(pattern) - LBound(pattern) + 1

    If maxBytes < patternLen Or patternLen <= 0 Then
        MemSearchPattern = NULL_PTR
        Exit Function
    End If

    ma.sa.pvData = startPtr
    ma.sa.cbElements = 1
    ma.sa.rgsabound0.cElements = maxBytes

    For i = 0 To maxBytes - patternLen
        found = True
        For j = 0 To patternLen - 1
            If ma.ac.dByte(i + j) <> pattern(LBound(pattern) + j) Then
                found = False
                Exit For
            End If
        Next j
        If found Then
            MemSearchPattern = startPtr + i
            GoTo Cleanup
        End If
    Next i

    MemSearchPattern = NULL_PTR

Cleanup:
    ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR
End Function

Public Sub MemSwap(ByVal ptr1 As LongPtr, ByVal ptr2 As LongPtr, ByVal bytesCount As Long)
    Static ma1 As MEMORY_ACCESSOR: If Not ma1.isSet Then InitMemoryAccessor ma1
    Static ma2 As MEMORY_ACCESSOR: If Not ma2.isSet Then InitMemoryAccessor ma2
    Dim temp As Currency
    Dim tempLong As Long
    Dim tempByte As Byte

    If bytesCount <= 0 Or ptr1 = ptr2 Then Exit Sub

    ma1.sa.pvData = ptr1
    ma2.sa.pvData = ptr2
    ma1.sa.rgsabound0.cElements = 1
    ma2.sa.rgsabound0.cElements = 1

    Dim remaining As Long: remaining = bytesCount

    Do While remaining >= 8
        ma1.sa.cbElements = 8
        ma2.sa.cbElements = 8
        temp = ma1.ac.dCur(0)
        ma1.ac.dCur(0) = ma2.ac.dCur(0)
        ma2.ac.dCur(0) = temp
        ma1.sa.pvData = ma1.sa.pvData + 8
        ma2.sa.pvData = ma2.sa.pvData + 8
        remaining = remaining - 8
    Loop

    Do While remaining >= 4
        ma1.sa.cbElements = 4
        ma2.sa.cbElements = 4
        tempLong = ma1.ac.dLong(0)
        ma1.ac.dLong(0) = ma2.ac.dLong(0)
        ma2.ac.dLong(0) = tempLong
        ma1.sa.pvData = ma1.sa.pvData + 4
        ma2.sa.pvData = ma2.sa.pvData + 4
        remaining = remaining - 4
    Loop

    ma1.sa.cbElements = 1
    ma2.sa.cbElements = 1
    Do While remaining > 0
        tempByte = ma1.ac.dByte(0)
        ma1.ac.dByte(0) = ma2.ac.dByte(0)
        ma2.ac.dByte(0) = tempByte
        ma1.sa.pvData = ma1.sa.pvData + 1
        ma2.sa.pvData = ma2.sa.pvData + 1
        remaining = remaining - 1
    Loop

    ma1.sa.rgsabound0.cElements = 0: ma1.sa.pvData = NULL_PTR
    ma2.sa.rgsabound0.cElements = 0: ma2.sa.pvData = NULL_PTR
End Sub

Public Function MemToHex(ByVal memAddress As LongPtr, ByVal bytesCount As Long) As String
    Static ma As MEMORY_ACCESSOR: If Not ma.isSet Then InitMemoryAccessor ma
    Dim i As Long
    Dim hexChars() As String

    If bytesCount <= 0 Then Exit Function

    ReDim hexChars(0 To bytesCount - 1)

    ma.sa.pvData = memAddress
    ma.sa.cbElements = 1
    ma.sa.rgsabound0.cElements = bytesCount

    For i = 0 To bytesCount - 1
        hexChars(i) = Right$("0" & Hex$(ma.ac.dByte(i)), 2)
    Next i

    ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR

    MemToHex = Join(hexChars, " ")
End Function

Public Sub HexToMem(ByVal hexString As String, ByVal destPtr As LongPtr)
    Static ma As MEMORY_ACCESSOR: If Not ma.isSet Then InitMemoryAccessor ma
    Dim cleanHex As String
    Dim i As Long
    Dim byteCount As Long

    cleanHex = Replace(hexString, " ", "")
    cleanHex = Replace(cleanHex, "-", "")

    byteCount = Len(cleanHex) \ 2
    If byteCount = 0 Then Exit Sub

    ma.sa.pvData = destPtr
    ma.sa.cbElements = 1
    ma.sa.rgsabound0.cElements = byteCount

    For i = 0 To byteCount - 1
        ma.ac.dByte(i) = CByte("&H" & Mid$(cleanHex, i * 2 + 1, 2))
    Next i

    ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR
End Sub

Public Function MemChecksum(ByVal memAddress As LongPtr, ByVal bytesCount As Long) As Long
    Static ma As MEMORY_ACCESSOR: If Not ma.isSet Then InitMemoryAccessor ma
    Dim i As Long
    Dim sum As Long

    If bytesCount <= 0 Then Exit Function

    ma.sa.pvData = memAddress
    ma.sa.cbElements = 1
    ma.sa.rgsabound0.cElements = bytesCount

    sum = 0
    For i = 0 To bytesCount - 1
        sum = sum Xor (CLng(ma.ac.dByte(i)) * (2 ^ (i Mod 24)))
    Next i

    ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR

    MemChecksum = sum
End Function

Public Function MemReverse(ByVal memAddress As LongPtr, ByVal bytesCount As Long) As Boolean
    Static ma As MEMORY_ACCESSOR: If Not ma.isSet Then InitMemoryAccessor ma
    Dim i As Long
    Dim j As Long
    Dim temp As Byte

    If bytesCount <= 1 Then
        MemReverse = True
        Exit Function
    End If

    ma.sa.pvData = memAddress
    ma.sa.cbElements = 1
    ma.sa.rgsabound0.cElements = bytesCount

    i = 0
    j = bytesCount - 1
    Do While i < j
        temp = ma.ac.dByte(i)
        ma.ac.dByte(i) = ma.ac.dByte(j)
        ma.ac.dByte(j) = temp
        i = i + 1
        j = j - 1
    Loop

    ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR

    MemReverse = True
End Function

Public Function ReadString(ByVal memAddress As LongPtr, Optional ByVal maxLen As Long = 260) As String
    Static ma As MEMORY_ACCESSOR: If Not ma.isSet Then InitMemoryAccessor ma
    Dim i As Long
    Dim chars() As Byte
    Dim actualLen As Long

    If maxLen <= 0 Then Exit Function

    ma.sa.pvData = memAddress
    ma.sa.cbElements = 1
    ma.sa.rgsabound0.cElements = maxLen

    actualLen = 0
    For i = 0 To maxLen - 1
        If ma.ac.dByte(i) = 0 Then Exit For
        actualLen = actualLen + 1
    Next i

    If actualLen = 0 Then
        ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR
        Exit Function
    End If

    ReDim chars(0 To actualLen - 1)
    For i = 0 To actualLen - 1
        chars(i) = ma.ac.dByte(i)
    Next i

    ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR

    ReadString = StrConv(chars, vbUnicode)
End Function

Public Function ReadWString(ByVal memAddress As LongPtr, Optional ByVal maxChars As Long = 260) As String
    Static ma As MEMORY_ACCESSOR: If Not ma.isSet Then InitMemoryAccessor ma
    Dim i As Long
    Dim actualLen As Long

    If maxChars <= 0 Then Exit Function

    ma.sa.pvData = memAddress
    ma.sa.cbElements = 2
    ma.sa.rgsabound0.cElements = maxChars

    actualLen = 0
    For i = 0 To maxChars - 1
        If ma.ac.dInt(i) = 0 Then Exit For
        actualLen = actualLen + 1
    Next i

    ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR

    If actualLen = 0 Then Exit Function

    ReadWString = Space$(actualLen)
    MemCopy StrPtr(ReadWString), memAddress, actualLen * 2
End Function

Public Sub WriteString(ByVal memAddress As LongPtr, ByVal s As String)
    Static ma As MEMORY_ACCESSOR: If Not ma.isSet Then InitMemoryAccessor ma
    Dim bytes() As Byte
    Dim i As Long

    If Len(s) = 0 Then
        MemByte(memAddress) = 0
        Exit Sub
    End If

    bytes = StrConv(s, vbFromUnicode)

    ma.sa.pvData = memAddress
    ma.sa.cbElements = 1
    ma.sa.rgsabound0.cElements = UBound(bytes) + 2

    For i = 0 To UBound(bytes)
        ma.ac.dByte(i) = bytes(i)
    Next i
    ma.ac.dByte(UBound(bytes) + 1) = 0

    ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR
End Sub

Public Sub WriteWString(ByVal memAddress As LongPtr, ByVal s As String)
    Dim byteLen As Long: byteLen = LenB(s)
    If byteLen > 0 Then MemCopy memAddress, StrPtr(s), byteLen
    MemInt(memAddress + byteLen) = 0
End Sub

Public Function MemXor(ByVal memAddress As LongPtr, ByVal bytesCount As Long, ByVal xorKey As Byte) As Boolean
    Static ma As MEMORY_ACCESSOR: If Not ma.isSet Then InitMemoryAccessor ma
    Dim i As Long

    If bytesCount <= 0 Then
        MemXor = True
        Exit Function
    End If

    ma.sa.pvData = memAddress
    ma.sa.cbElements = 1
    ma.sa.rgsabound0.cElements = bytesCount

    For i = 0 To bytesCount - 1
        ma.ac.dByte(i) = ma.ac.dByte(i) Xor xorKey
    Next i

    ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR

    MemXor = True
End Function

Public Function MemXorBlock(ByVal memAddress As LongPtr, ByVal bytesCount As Long, ByRef xorKey() As Byte) As Boolean
    Static ma As MEMORY_ACCESSOR: If Not ma.isSet Then InitMemoryAccessor ma
    Dim i As Long
    Dim keyLen As Long
    Dim keyIdx As Long

    keyLen = UBound(xorKey) - LBound(xorKey) + 1

    If bytesCount <= 0 Or keyLen <= 0 Then
        MemXorBlock = True
        Exit Function
    End If

    ma.sa.pvData = memAddress
    ma.sa.cbElements = 1
    ma.sa.rgsabound0.cElements = bytesCount

    keyIdx = 0
    For i = 0 To bytesCount - 1
        ma.ac.dByte(i) = ma.ac.dByte(i) Xor xorKey(LBound(xorKey) + keyIdx)
        keyIdx = (keyIdx + 1) Mod keyLen
    Next i

    ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR

    MemXorBlock = True
End Function

Public Function BytesToArray(ByVal memAddress As LongPtr, ByVal bytesCount As Long) As Byte()
    Static ma As MEMORY_ACCESSOR: If Not ma.isSet Then InitMemoryAccessor ma
    Dim result() As Byte

    If bytesCount <= 0 Then Exit Function

    ReDim result(0 To bytesCount - 1)
    MemCopy VarPtr(result(0)), memAddress, bytesCount

    BytesToArray = result
End Function

Public Function ArrayToMem(ByRef arr() As Byte, ByVal destPtr As LongPtr) As Long
    Dim byteCount As Long

    byteCount = UBound(arr) - LBound(arr) + 1
    If byteCount <= 0 Then Exit Function

    MemCopy destPtr, VarPtr(arr(LBound(arr))), byteCount

    ArrayToMem = byteCount
End Function

Public Function MemDump(ByVal memAddress As LongPtr, ByVal bytesCount As Long, Optional ByVal bytesPerLine As Long = 16) As String
    Static ma As MEMORY_ACCESSOR: If Not ma.isSet Then InitMemoryAccessor ma
    Dim lines() As String
    Dim lineCount As Long
    Dim i As Long
    Dim j As Long
    Dim hexPart As String
    Dim asciiPart As String
    Dim b As Byte

    If bytesCount <= 0 Then Exit Function

    lineCount = (bytesCount + bytesPerLine - 1) \ bytesPerLine
    ReDim lines(0 To lineCount - 1)

    ma.sa.pvData = memAddress
    ma.sa.cbElements = 1
    ma.sa.rgsabound0.cElements = bytesCount

    For i = 0 To lineCount - 1
        hexPart = ""
        asciiPart = ""

        #If Win64 Then
            lines(i) = Right$("0000000000000000" & Hex$(memAddress + i * bytesPerLine), 16) & "  "
        #Else
            lines(i) = Right$("00000000" & Hex$(memAddress + i * bytesPerLine), 8) & "  "
        #End If

        For j = 0 To bytesPerLine - 1
            If i * bytesPerLine + j < bytesCount Then
                b = ma.ac.dByte(i * bytesPerLine + j)
                hexPart = hexPart & Right$("0" & Hex$(b), 2) & " "
                If b >= 32 And b <= 126 Then
                    asciiPart = asciiPart & Chr$(b)
                Else
                    asciiPart = asciiPart & "."
                End If
            Else
                hexPart = hexPart & "   "
            End If
        Next j

        lines(i) = lines(i) & hexPart & " |" & asciiPart & "|"
    Next i

    ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR

    MemDump = Join(lines, vbNewLine)
End Function

Public Function MemObj(ByVal memAddress As LongPtr) As Object
    If memAddress = NULL_PTR Then Exit Function

    #If Mac Or TWINBASIC Or (VBA7 = 0) Then
        Dim obj As Object
        #If TWINBASIC Then
            PutMemPtr ByVal VarPtr(obj), memAddress
        #Else
            CopyMemory obj, memAddress, PTR_SIZE
        #End If
        Set MemObj = obj
        CopyMemory obj, NULL_PTR, PTR_SIZE
    #Else
        Static ma As MEMORY_ACCESSOR: If Not ma.isSet Then InitMemoryAccessor ma
        ma.sa.pvData = VarPtr(memAddress): ma.sa.rgsabound0.cElements = 1
        Set MemObj = ma.ac.dObj(0)
        ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR
    #End If
End Function

Public Function UnsignedAdd(ByVal unsignedPtr As LongPtr, ByVal signedOffset As LongPtr) As LongPtr
    #If Win64 Then
        Const minNegative As LongLong = &H8000000000000000^
    #Else
        Const minNegative As Long = &H80000000
    #End If
    UnsignedAdd = ((unsignedPtr Xor minNegative) + signedOffset) Xor minNegative
End Function

Public Sub RedirectInstance(ByRef funcReturn As Variant _
                          , ByVal funcReturnPtr As LongPtr _
                          , ByVal currentInstance As stdole.IUnknown _
                          , ByVal targetInstance As stdole.IUnknown)
    Const methodName As String = "RedirectInstance"
    If currentInstance Is Nothing Or targetInstance Is Nothing Then
        Err.Raise 91, methodName, "Object not set"
    End If

    Static ma As MEMORY_ACCESSOR
    Dim originalPtr As LongPtr
    Dim newPtr As LongPtr
    Dim ptr As LongPtr
    Dim swapAddress As LongPtr
    Dim temp As Object

    Set temp = currentInstance: originalPtr = ObjPtr(temp)
    Set temp = targetInstance:  newPtr = ObjPtr(temp)

    If Not ma.isSet Then
        InitMemoryAccessor ma
        ma.sa.cbElements = PTR_SIZE
    End If
    ma.sa.pvData = originalPtr
    ma.sa.rgsabound0.cElements = 1
    ptr = ma.ac.dPtr(0)
    ma.sa.pvData = newPtr
    If ptr <> ma.ac.dPtr(0) Then
        ma.sa.rgsabound0.cElements = 0
        ma.sa.pvData = NULL_PTR
        Err.Raise 5, methodName, "Expected same VB class"
    End If

    #If Win64 Then
        Const memOffsetNonVariant As LongLong = PTR_SIZE
        Const memOffsetVariant As LongLong = PTR_SIZE * 3
    #Else
        Const memOffsetNonVariant As Long = PTR_SIZE * 28
        Const memOffsetVariant As Long = PTR_SIZE * 31
    #End If

    ma.sa.pvData = VarPtr(funcReturn)
    If (ma.ac.dInt(0) And VT_BYREF) = 0 Then
        ma.sa.pvData = ma.sa.pvData + memOffsetVariant
        #If Win64 = 0 Then
            ma.sa.pvData = ma.ac.dPtr(0) + PTR_SIZE * 2
        #End If
        If ma.ac.dPtr(0) = originalPtr Then swapAddress = ma.sa.pvData
    Else
        Const variantPtrOffset As Long = 8
        Dim vt As Integer: vt = ma.ac.dInt(0) Xor VT_BYREF

        If (vt = vbObject) Or (vt = vbDataObject) Then
            ptr = funcReturnPtr
        Else
            ma.sa.pvData = ma.sa.pvData + variantPtrOffset
            ptr = ma.ac.dPtr(0)
            #If Mac Or (Win64 = 0) Then
                ptr = ptr - (ptr Mod PTR_SIZE)
            #End If
        End If

        ma.sa.pvData = ptr + memOffsetNonVariant
        #If Win64 = 0 Then
            If vt = vbCurrency Or vt = vbDate Or vt = vbDouble Then
                ma.sa.pvData = ma.sa.pvData + PTR_SIZE
            End If
            ma.sa.pvData = ma.ac.dPtr(0) + PTR_SIZE * 2
        #End If
        If ma.ac.dPtr(0) = originalPtr Then swapAddress = ma.sa.pvData
    End If

    If swapAddress = NULL_PTR Then
        ma.sa.rgsabound0.cElements = 0
        ma.sa.pvData = NULL_PTR
        Err.Raise 5, methodName, "Not called from class Func or UDT Func Return"
    End If

    ma.sa.pvData = swapAddress
    ma.ac.dPtr(0) = newPtr
    ma.sa.rgsabound0.cElements = 0
    ma.sa.pvData = NULL_PTR
End Sub

Public Function GetDefaultInterface(ByVal obj As stdole.IUnknown) As Object
    Set GetDefaultInterface = obj
End Function

Public Function VarPtrArr(ByRef arr As Variant) As LongPtr
    Static ma As MEMORY_ACCESSOR: If Not ma.isSet Then InitMemoryAccessor ma
    ma.sa.pvData = VarPtr(arr): ma.sa.rgsabound0.cElements = 1

    Dim vt As VbVarType: vt = ma.ac.dInt(0)
    If vt And vbArray Then
        Const pArrayOffset As Long = 8
        VarPtrArr = ma.sa.pvData + pArrayOffset
        If vt And VT_BYREF Then
            ma.sa.pvData = VarPtrArr
            VarPtrArr = ma.ac.dPtr(0)
        End If
    Else
        ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR
        Err.Raise 5, "VarPtrArr", "Array required"
    End If
    ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR
End Function

Public Function ArrPtr(ByRef arr As Variant) As LongPtr
    Static ma As MEMORY_ACCESSOR: If Not ma.isSet Then InitMemoryAccessor ma
    ma.sa.pvData = VarPtr(arr): ma.sa.rgsabound0.cElements = 1

    Dim vt As VbVarType: vt = ma.ac.dInt(0)
    If vt And vbArray Then
        Const pArrayOffset As Long = 8
        ma.sa.pvData = ma.sa.pvData + pArrayOffset
        ArrPtr = ma.ac.dPtr(0)
        If vt And VT_BYREF Then
            ma.sa.pvData = ArrPtr
            ArrPtr = ma.ac.dPtr(0)
        End If
    Else
        ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR
        Err.Raise 5, "ArrPtr", "Array required"
    End If
    ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR
End Function

#If (Mac = 0) And (TWINBASIC = 0) And VBA7 Then
Public Sub MemCopy(ByVal destinationPtr As LongPtr _
                 , ByVal sourcePtr As LongPtr _
                 , ByVal bytesCount As LongPtr)
    Const maxSizeSpeedGain As Long = &H2000000
    If bytesCount < 0 Or bytesCount > maxSizeSpeedGain Then
        CopyMemory ByVal destinationPtr, ByVal sourcePtr, bytesCount
        Exit Sub
    ElseIf destinationPtr = sourcePtr Then
        Exit Sub
    End If

    Static src As MEMORY_ACCESSOR
    Static trg As MEMORY_ACCESSOR
    Static byteMap(0 To 255) As ByteInfo
    Dim i As Long
    Dim j As Long

    If Not src.isSet Then
        InitMemoryAccessor src
        InitMemoryAccessor trg
        For i = &H1 To &HFF&
            With byteMap(i)
                For j = 0 To 7
                    .bit(j) = i And 2 ^ j
                Next j
            End With
        Next i
    End If

    src.sa.pvData = sourcePtr
    trg.sa.pvData = destinationPtr

    If bytesCount <= 8 Then
        src.sa.rgsabound0.cElements = 1
        trg.sa.rgsabound0.cElements = 1
        Select Case bytesCount
            Case 0: GoTo Clean
            Case 1: trg.ac.dByte(0) = src.ac.dByte(0): GoTo Clean
            Case 2: trg.ac.dInt(0) = src.ac.dInt(0): GoTo Clean
            Case 4: trg.ac.dLong(0) = src.ac.dLong(0): GoTo Clean
            Case 8: trg.ac.dCur(0) = src.ac.dCur(0): GoTo Clean
        End Select
    End If

    Dim b As Long: b = CLng(bytesCount)
    Dim chunk As Long
    Dim overlapR As Boolean

    overlapR = (destinationPtr > sourcePtr) And (sourcePtr + b > destinationPtr)

    If b And &H7FFF8000 Then
        src.sa.cbElements = &H8000&
        trg.sa.cbElements = &H8000&
        src.sa.rgsabound0.cElements = b \ src.sa.cbElements
        trg.sa.rgsabound0.cElements = src.sa.rgsabound0.cElements

        chunk = src.sa.rgsabound0.cElements * src.sa.cbElements
        b = b - chunk

        If overlapR Then
            src.sa.pvData = src.sa.pvData + b
            trg.sa.pvData = trg.sa.pvData + b
            For i = src.sa.rgsabound0.cElements - 1 To 0 Step -1
                trg.ac.s32K(i) = src.ac.s32K(i)
            Next i
        Else
            For i = 0 To src.sa.rgsabound0.cElements - 1
                trg.ac.b32K(i) = src.ac.b32K(i)
            Next i
            src.sa.pvData = src.sa.pvData + chunk
            trg.sa.pvData = trg.sa.pvData + chunk
        End If
        chunk = &H8000&
    ElseIf overlapR Then
        src.sa.pvData = src.sa.pvData + b
        trg.sa.pvData = trg.sa.pvData + b
    End If
    src.sa.rgsabound0.cElements = 1
    trg.sa.rgsabound0.cElements = 1

    i = b And &HFF&
    If i Then
        With byteMap(i)
            If overlapR Then
                If .bit(0) Then src.sa.pvData = src.sa.pvData - 1: trg.sa.pvData = trg.sa.pvData - 1: trg.ac.dByte(0) = src.ac.dByte(0)
                If .bit(1) Then src.sa.pvData = src.sa.pvData - 2: trg.sa.pvData = trg.sa.pvData - 2: trg.ac.dInt(0) = src.ac.dInt(0)
                If .bit(2) Then src.sa.pvData = src.sa.pvData - 4: trg.sa.pvData = trg.sa.pvData - 4: trg.ac.dLong(0) = src.ac.dLong(0)
                If .bit(3) Then src.sa.pvData = src.sa.pvData - 8: trg.sa.pvData = trg.sa.pvData - 8: trg.ac.dCur(0) = src.ac.dCur(0)
                If .bit(4) Then src.sa.pvData = src.sa.pvData - 16: trg.sa.pvData = trg.sa.pvData - 16: trg.ac.s16(0) = src.ac.s16(0)
                If .bit(5) Then src.sa.pvData = src.sa.pvData - 32: trg.sa.pvData = trg.sa.pvData - 32: trg.ac.s32(0) = src.ac.s32(0)
                If .bit(6) Then src.sa.pvData = src.sa.pvData - 64: trg.sa.pvData = trg.sa.pvData - 64: trg.ac.s64(0) = src.ac.s64(0)
                If .bit(7) Then src.sa.pvData = src.sa.pvData - 128: trg.sa.pvData = trg.sa.pvData - 128: trg.ac.s128(0) = src.ac.s128(0)
            Else
                If .bit(0) Then trg.ac.dByte(0) = src.ac.dByte(0): src.sa.pvData = src.sa.pvData + 1: trg.sa.pvData = trg.sa.pvData + 1
                If .bit(1) Then trg.ac.dInt(0) = src.ac.dInt(0): src.sa.pvData = src.sa.pvData + 2: trg.sa.pvData = trg.sa.pvData + 2
                If .bit(2) Then trg.ac.dLong(0) = src.ac.dLong(0): src.sa.pvData = src.sa.pvData + 4: trg.sa.pvData = trg.sa.pvData + 4
                If .bit(3) Then trg.ac.dCur(0) = src.ac.dCur(0): src.sa.pvData = src.sa.pvData + 8: trg.sa.pvData = trg.sa.pvData + 8
                If .bit(4) Then trg.ac.b16(0) = src.ac.b16(0): src.sa.pvData = src.sa.pvData + 16: trg.sa.pvData = trg.sa.pvData + 16
                If .bit(5) Then trg.ac.b32(0) = src.ac.b32(0): src.sa.pvData = src.sa.pvData + 32: trg.sa.pvData = trg.sa.pvData + 32
                If .bit(6) Then trg.ac.b64(0) = src.ac.b64(0): src.sa.pvData = src.sa.pvData + 64: trg.sa.pvData = trg.sa.pvData + 64
                If .bit(7) Then trg.ac.b128(0) = src.ac.b128(0): src.sa.pvData = src.sa.pvData + 128: trg.sa.pvData = trg.sa.pvData + 128
            End If
        End With
    End If

    i = (b And &H7F00&) / &H100&
    If i Then
        With byteMap(i)
            If overlapR Then
                If .bit(0) Then src.sa.pvData = src.sa.pvData - 256: trg.sa.pvData = trg.sa.pvData - 256: trg.ac.s256(0) = src.ac.s256(0)
                If .bit(1) Then src.sa.pvData = src.sa.pvData - 512: trg.sa.pvData = trg.sa.pvData - 512: trg.ac.s512(0) = src.ac.s512(0)
                If .bit(2) Then src.sa.pvData = src.sa.pvData - 1024: trg.sa.pvData = trg.sa.pvData - 1024: trg.ac.s1K(0) = src.ac.s1K(0)
                If .bit(3) Then src.sa.pvData = src.sa.pvData - 2048: trg.sa.pvData = trg.sa.pvData - 2048: trg.ac.s2K(0) = src.ac.s2K(0)
                If .bit(4) Then src.sa.pvData = src.sa.pvData - 4096: trg.sa.pvData = trg.sa.pvData - 4096: trg.ac.s4K(0) = src.ac.s4K(0)
                If .bit(5) Then src.sa.pvData = src.sa.pvData - 8192: trg.sa.pvData = trg.sa.pvData - 8192: trg.ac.s8K(0) = src.ac.s8K(0)
                If .bit(6) Then src.sa.pvData = src.sa.pvData - 16384: trg.sa.pvData = trg.sa.pvData - 16384: trg.ac.s16K(0) = src.ac.s16K(0)
            Else
                If .bit(0) Then trg.ac.b256(0) = src.ac.b256(0): src.sa.pvData = src.sa.pvData + 256: trg.sa.pvData = trg.sa.pvData + 256
                If .bit(1) Then trg.ac.b512(0) = src.ac.b512(0): src.sa.pvData = src.sa.pvData + 512: trg.sa.pvData = trg.sa.pvData + 512
                If .bit(2) Then trg.ac.b1K(0) = src.ac.b1K(0): src.sa.pvData = src.sa.pvData + 1024: trg.sa.pvData = trg.sa.pvData + 1024
                If .bit(3) Then trg.ac.b2K(0) = src.ac.b2K(0): src.sa.pvData = src.sa.pvData + 2048: trg.sa.pvData = trg.sa.pvData + 2048
                If .bit(4) Then trg.ac.b4K(0) = src.ac.b4K(0): src.sa.pvData = src.sa.pvData + 4096: trg.sa.pvData = trg.sa.pvData + 4096
                If .bit(5) Then trg.ac.b8K(0) = src.ac.b8K(0): src.sa.pvData = src.sa.pvData + 8192: trg.sa.pvData = trg.sa.pvData + 8192
                If .bit(6) Then trg.ac.b16K(0) = src.ac.b16K(0): src.sa.pvData = src.sa.pvData + 16384: trg.sa.pvData = trg.sa.pvData + 16384
            End If
        End With
    End If
Clean:
    src.sa.rgsabound0.cElements = 0
    trg.sa.rgsabound0.cElements = 0
    src.sa.pvData = NULL_PTR
    trg.sa.pvData = NULL_PTR
End Sub
#End If

Public Sub CloneParamArray(ByVal paramPtr As LongPtr, ByRef out() As Variant)
    Static ma As MEMORY_ACCESSOR: If Not ma.isSet Then InitMemoryAccessor ma
    Dim v As Variant
    Dim sa As SAFEARRAY_1D

    MemCopy VarPtr(sa), paramPtr, LenB(sa)
    v = VarPtr(sa)
    sa.cLocks = 1

    ma.sa.pvData = VarPtr(v): ma.sa.rgsabound0.cElements = 1
    ma.ac.dInt(0) = vbArray + vbVariant
    out = v
    ma.ac.dInt(0) = vbLongPtr
    ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR
End Sub

Public Function GetArrayByRef(ByRef arr As Variant) As Variant
    If IsArray(arr) Then
        GetArrayByRef = VarPtrArr(arr)
        MemInt(VarPtr(GetArrayByRef)) = VarType(arr) Or VT_BYREF
    Else
        Err.Raise 5, "GetArrayByRef", "Array required"
    End If
End Function

Public Function StringToIntegers(ByRef s As String _
                               , Optional ByVal startIndex As Long = 1 _
                               , Optional ByVal outLength As Long = -1 _
                               , Optional ByVal outLowBound As Long = 0) As Integer()
    Static ma As MEMORY_ACCESSOR: If Not ma.isSet Then InitMemoryAccessor ma
    Const methodName As String = "StringToIntegers"
    Dim cLen As Long: cLen = Len(s)

    If startIndex < 1 Then
        Err.Raise 9, methodName, "Invalid Start Index"
    ElseIf outLength < -1 Then
        Err.Raise 5, methodName, "Invalid Length for output"
    ElseIf outLength = -1 Or startIndex + outLength - 1 > cLen Then
        outLength = cLen - startIndex + 1
        If outLength < 0 Then outLength = 0
    End If

    ma.sa.pvData = StrPtr(s) + (startIndex - 1) * INT_SIZE
    ma.sa.cbElements = INT_SIZE
    ma.sa.rgsabound0.lLbound = outLowBound
    ma.sa.rgsabound0.cElements = outLength
    StringToIntegers = ma.ac.dInt
    ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR
End Function

Public Function IntegersToString(ByRef ints() As Integer _
                               , Optional ByVal startIndex As Long = 0 _
                               , Optional ByVal outLength As Long = -1) As String
    Static ma As MEMORY_ACCESSOR: If Not ma.isSet Then InitMemoryAccessor ma
    Const methodName As String = "IntegersToString"

    If GetArrayDimsCount(ints) <> 1 Then
        Err.Raise 5, methodName, "Expected 1D Array of Integers"
    ElseIf startIndex < LBound(ints) Then
        Err.Raise 9, methodName, "Invalid Start Index"
    ElseIf outLength < -1 Then
        Err.Raise 5, methodName, "Invalid Length for output"
    ElseIf outLength = -1 Or startIndex + outLength - 1 > UBound(ints) Then
        outLength = UBound(ints) - startIndex + 1
        If outLength < 0 Then Exit Function
    End If

    ma.sa.pvData = VarPtr(ints(startIndex))
    ma.sa.cbElements = BYTE_SIZE
    ma.sa.rgsabound0.cElements = outLength * INT_SIZE
    IntegersToString = ma.ac.dByte
    ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR
End Function

Public Function EmptyArray(ByVal numberOfDimensions As Long _
                         , ByVal vType As VbVarType) As Variant
    Const methodName As String = "EmptyArray"
    Const MAX_DIMENSION As Long = 60

    If numberOfDimensions < 1 Or numberOfDimensions > MAX_DIMENSION Then
        Err.Raise 5, methodName, "Invalid number of dimensions"
    End If
    Select Case vType
    Case vbByte, vbInteger, vbLong, vbLongLong
    Case vbCurrency, vbDecimal, vbDouble, vbSingle, vbDate
    Case vbBoolean, vbString, vbObject, vbDataObject, vbVariant
    Case Else
        Err.Raise 13, methodName, "Type mismatch"
    End Select

    Static fakeSafeArray() As Long
    Static ma As MEMORY_ACCESSOR
    Static v As Variant
    #If Win64 Then
        Const safeArraySize = 6
    #Else
        Const safeArraySize = 4
    #End If
    Const fFeaturesHi As Long = FADF_HAVEVARTYPE * &H10000
    Dim i As Long

    If Not ma.isSet Then
        InitMemoryAccessor ma
        ReDim fakeSafeArray(0 To safeArraySize + MAX_DIMENSION * 2 - 1)
        fakeSafeArray(1) = 1
        v = VarPtr(fakeSafeArray(0))

        For i = safeArraySize To UBound(fakeSafeArray, 1) Step 2
            fakeSafeArray(i) = 1
        Next i
    End If
    fakeSafeArray(0) = fFeaturesHi + numberOfDimensions
    i = safeArraySize + (numberOfDimensions - 1) * 2

    fakeSafeArray(i) = 0
    ma.sa.pvData = VarPtr(v): ma.sa.rgsabound0.cElements = 1
    ma.ac.dInt(0) = vbArray + vType
    EmptyArray = v
    ma.ac.dInt(0) = vbLongPtr
    ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR
    fakeSafeArray(i) = 1
End Function

Public Sub UpdateLBound(ByRef arr As Variant _
                      , ByVal dimension As Long _
                      , ByVal newLB As Long)
    Const bOffset As Long = rgsaboundOffset + 4
    Const methodName As String = "UpdateLBound"
    Const maxL As Long = &H7FFFFFFF
    Dim dimensionCount As Long: dimensionCount = GetArrayDimsCount(arr)

    If dimension < 1 Or dimension > dimensionCount Then
        Err.Raise 5, methodName, "Invalid dimension or not array"
    ElseIf maxL - UBound(arr, dimension) + LBound(arr, dimension) < newLB Then
        Err.Raise 5, methodName, "New bound overflow"
    End If
    MemLong(ArrPtr(arr) + bOffset + (dimensionCount - dimension) * 8) = newLB
End Sub

Private Function GetArrayDimsCount(ByRef arr As Variant) As Long
    Static ma As MEMORY_ACCESSOR: If Not ma.isSet Then InitMemoryAccessor ma

    ma.sa.pvData = VarPtr(arr)
    ma.sa.cbElements = INT_SIZE
    ma.sa.rgsabound0.cElements = 1
    Dim vt As Integer: vt = ma.ac.dInt(0)
    ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR

    If (vt And vbArray) = 0 Then Exit Function

    Dim ppSA As LongPtr
    ppSA = VarPtr(arr) + 8

    If vt And VT_BYREF Then
        ma.sa.pvData = ppSA
        ma.sa.cbElements = PTR_SIZE
        ma.sa.rgsabound0.cElements = 1
        ppSA = ma.ac.dPtr(0)
        ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR
    End If

    ma.sa.pvData = ppSA
    ma.sa.cbElements = PTR_SIZE
    ma.sa.rgsabound0.cElements = 1
    Dim saPtr As LongPtr: saPtr = ma.ac.dPtr(0)
    ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR

    If saPtr = NULL_PTR Then Exit Function

    ma.sa.pvData = saPtr
    ma.sa.cbElements = INT_SIZE
    ma.sa.rgsabound0.cElements = 1
    GetArrayDimsCount = ma.ac.dInt(0)
    ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR
End Function

Public Sub MemZero(ByVal destinationPtr As LongPtr, ByVal bytesCount As LongPtr)
    MemFill destinationPtr, bytesCount, 0
End Sub

#If Mac Or VBA7 Then
Public Sub MemFill(ByVal destinationPtr As LongPtr _
                 , ByVal bytesCount As LongPtr _
                 , ByVal fillByte As Byte)
    #If Mac Then
        FillMemory ByVal destinationPtr, fillByte, bytesCount
    #Else
        If bytesCount = 0 Then Exit Sub
        Const maxSizeSpeedGain As Long = &H100000
        If bytesCount < 0 Or bytesCount > maxSizeSpeedGain Then
            FillMemory ByVal destinationPtr, bytesCount, fillByte
            Exit Sub
        End If

        Const maxSizeMidB As Long = &H2000
        Dim bytesLeft As Long
        Dim bytes As Long
        Dim chunk As Long

        If bytesCount > maxSizeMidB Then
            bytes = maxSizeMidB
            bytesLeft = CLng(bytesCount) - maxSizeMidB
            chunk = maxSizeMidB
        Else
            bytes = CLng(bytesCount)
        End If

        Const bstrPrefixSize As Long = 4
        Static ma As MEMORY_ACCESSOR: If Not ma.isSet Then InitMemoryAccessor ma
        Dim i As Long

        ma.sa.pvData = destinationPtr
        ma.sa.cbElements = BYTE_SIZE
        If bytes > 5 Then
            Dim s As String

            ma.sa.rgsabound0.cElements = 5
            ma.ac.dLong(0) = bytes - bstrPrefixSize
            ma.ac.dByte(4) = fillByte
            ma.sa.pvData = VarPtr(s)
            ma.ac.dPtr(0) = destinationPtr + bstrPrefixSize
            MidB$(s, 2) = s
            ma.ac.dPtr(0) = NULL_PTR
            ma.sa.pvData = destinationPtr
            bytes = 4
        End If
        ma.sa.rgsabound0.cElements = bytes
        For i = 0 To bytes - 1
            ma.ac.dByte(i) = fillByte
        Next i
        ma.sa.rgsabound0.cElements = 0
        ma.sa.pvData = NULL_PTR

        Do While bytesLeft > 0
            If chunk > bytesLeft Then bytes = bytesLeft Else bytes = chunk
            MemCopy destinationPtr + chunk, destinationPtr, bytes
            bytesLeft = bytesLeft - bytes
            chunk = chunk * 2
        Loop
    #End If
End Sub
#End If

Public Function MemAlloc(ByVal byteSize As Long) As LongPtr
    Static ma As MEMORY_ACCESSOR: If Not ma.isSet Then InitMemoryAccessor ma
    Static i As stdole.IEnumVARIANT
    Static nextItemAddr As LongPtr
    Static arr() As Byte
    Static arrAddr As LongPtr
    Dim aPtr As LongPtr
    Dim pvPtr As LongPtr

    If byteSize < 1 Then Exit Function
    ReDim arr(0 To byteSize - 1)

    pvPtr = VarPtr(arr(0))

    If nextItemAddr = NULL_PTR Then
        Set i = m_allocMemory.[_NewEnum]
        nextItemAddr = ObjPtr(i) + PTR_SIZE * 2
        arrAddr = VarPtrArr(arr)
    End If
    If m_allocMemory.count = 0 Then
        m_allocMemory.Add Empty, CStr(pvPtr)
    Else
        m_allocMemory.Add Empty, CStr(pvPtr), 1
    End If
    i.Reset

    ma.sa.pvData = arrAddr
    ma.sa.rgsabound0.cElements = 1
    aPtr = ma.ac.dPtr(0)
    ma.ac.dPtr(0) = NULL_PTR
    ma.sa.pvData = nextItemAddr
    ma.sa.pvData = ma.ac.dPtr(0) + 8
    ma.ac.dPtr(0) = aPtr
    ma.sa.pvData = ma.sa.pvData - 8
    ma.ac.dInt(0) = vbArray + vbByte
    ma.sa.rgsabound0.cElements = 0
    ma.sa.pvData = NULL_PTR

    MemAlloc = pvPtr
End Function

Public Sub MemFree(ByVal memAddress As LongPtr)
    Dim key As String: key = CStr(memAddress)
    Dim i As Long

    For i = 1 To m_allocMemory.count
        If CollectionKeyExists(key) Then
            m_allocMemory.Remove key
            Exit For
        End If
    Next i
End Sub

Private Function CollectionKeyExists(ByVal key As String) As Boolean
    Static ma As MEMORY_ACCESSOR: If Not ma.isSet Then InitMemoryAccessor ma
    Dim i As stdole.IEnumVARIANT
    Dim itemPtr As LongPtr
    Dim nextPtr As LongPtr
    Dim keyPtr As LongPtr
    Dim keyLen As Long
    Dim testKey As String
    Dim j As Long

    If m_allocMemory.count = 0 Then Exit Function

    Set i = m_allocMemory.[_NewEnum]
    itemPtr = ObjPtr(i) + PTR_SIZE * 2

    ma.sa.pvData = itemPtr
    ma.sa.cbElements = PTR_SIZE
    ma.sa.rgsabound0.cElements = 1
    nextPtr = ma.ac.dPtr(0)
    ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR

    For j = 1 To m_allocMemory.count
        If nextPtr = NULL_PTR Then Exit For

        ma.sa.pvData = nextPtr + PTR_SIZE * 2
        ma.sa.cbElements = PTR_SIZE
        ma.sa.rgsabound0.cElements = 1
        keyPtr = ma.ac.dPtr(0)
        ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR

        If keyPtr <> NULL_PTR Then
            ma.sa.pvData = keyPtr - 4
            ma.sa.cbElements = LONG_SIZE
            ma.sa.rgsabound0.cElements = 1
            keyLen = ma.ac.dLong(0) \ 2
            ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR

            If keyLen > 0 And keyLen < 100 Then
                testKey = Space$(keyLen)
                MemCopy StrPtr(testKey), keyPtr, keyLen * 2

                If testKey = key Then
                    CollectionKeyExists = True
                    Exit Function
                End If
            End If
        End If

        ma.sa.pvData = nextPtr
        ma.sa.cbElements = PTR_SIZE
        ma.sa.rgsabound0.cElements = 1
        nextPtr = ma.ac.dPtr(0)
        ma.sa.rgsabound0.cElements = 0: ma.sa.pvData = NULL_PTR
    Next j

    CollectionKeyExists = False
End Function
