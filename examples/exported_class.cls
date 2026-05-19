VERSION 1.0 CLASS
BEGIN
  MultiUse = -1
END
Attribute VB_Name = "Counter"

Option Explicit

Private value As Long

Private Sub Class_Initialize()
    value = 1
End Sub

Private Sub Class_Terminate()
    Console.WriteLine("terminated")
End Sub

Public Property Get Item() As Long
    Attribute Item.VB_UserMemId = 0
    Item = value
End Property
