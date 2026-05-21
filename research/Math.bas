Attribute VB_Name = "Math"
' **************************************************************************
' Bedrock Team - Math Module
' **************************************************************************
' This module enhances VBA by adding essential mathematical functions that
' should be included by default, especially for applications in game development
' and other complex computational tasks. It provides key operations such as
' vector mathematics, distance and angle calculations, and 2D point rotations.
' Designed with performance in mind, this module ensures that calculations are
' fast and efficient, making it an indispensable tool for anyone working with
' advanced geometry manipulations. The Bedrock Team is dedicated to enhancing
' the capabilities of VBA by bridging gaps in its mathematical functionality,
' offering a more powerful and versatile toolkit for developers.
' --------------------------------------------------------------------------
' Team: Bedrock
' Last Update: 12/01/2024
' --------------------------------------------------------------------------
' References:
' - Khan Academy - Linear Algebra & Vectors
'   https://pt.khanacademy.org/math/linear-algebra
' - MathWorld - Vector Calculations
'   https://mathworld.wolfram.com/Vector.html
' - OpenGL Mathematics (GLM) Library for C++ (Conceptual Inspiration)
'   https://github.com/g-truc/glm
' - Rosetta Code - Geometry Examples
'   https://rosettacode.org/wiki/Category:Collision_detection
' - MathGL (Go 3D Math Library)
'   https://github.com/go-gl/mathgl
' - Game Physics Engine Development - Ian Millington
'   https://github.com/matheusportela/Poiesis/blob/master/references/Game%20Physics%20Engine%20Development%20-%20Ian%20Millington.pdf
' **************************************************************************

Option Explicit

Public Const PI As Double = 3.14159265358979

Private Function Overlap(projectionA As Variant, projectionB As Variant) As Boolean
    Overlap = Not (projectionA(1) < projectionB(0) Or projectionB(1) < projectionA(0))
End Function

Public Function CollisionForce(mass1 As Double, mass2 As Double, velocity1 As Double, velocity2 As Double) As Double
    CollisionForce = (2 * mass2 * (velocity1 - velocity2)) / (mass1 + mass2)
End Function

Public Function ElasticCollisionVelocity(mass1 As Double, mass2 As Double, velocity1 As Double, velocity2 As Double) As Variant
    Dim newVelocity1 As Double
    Dim newVelocity2 As Double
    Dim totalMass As Double
    
    totalMass = mass1 + mass2
    newVelocity1 = ((mass1 - mass2) * velocity1 + 2 * mass2 * velocity2) / totalMass
    newVelocity2 = ((mass2 - mass1) * velocity2 + 2 * mass1 * velocity1) / totalMass
    
    ElasticCollisionVelocity = Array(newVelocity1, newVelocity2)
End Function

Public Function DotProduct2D(v As Variant, axis As Variant) As Double
    DotProduct2D = (v(0) * axis(0)) + (v(1) * axis(1))
End Function

Private Function NearestPointOnSegment(x1 As Double, y1 As Double, x2 As Double, y2 As Double, px As Double, py As Double) As Variant
    Dim dx As Double, dy As Double
    Dim t As Double
    Dim nearestX As Double, nearestY As Double
    
    dx = x2 - x1
    dy = y2 - y1
    t = ((px - x1) * dx + (py - y1) * dy) / (dx ^ 2 + dy ^ 2)
    If t < 0 Then t = 0
    If t > 1 Then t = 1
    
    nearestX = x1 + t * dx
    nearestY = y1 + t * dy
    NearestPointOnSegment = Array(nearestX, nearestY)
End Function

Private Function NormalizeVector2D(v As Variant) As Variant
    Dim magnitude As Double
    magnitude = Sqr(v(0) ^ 2 + v(1) ^ 2)
    
    NormalizeVector2D = Array(v(0) / magnitude, v(1) / magnitude)
End Function

Public Function AngleBetweenVectors2D(vx1 As Double, vy1 As Double, vx2 As Double, vy2 As Double) As Double
    Dim DotProduct As Double
    Dim magnitude1 As Double
    Dim magnitude2 As Double
    
    DotProduct = (vx1 * vx2) + (vy1 * vy2)
    magnitude1 = Sqr(vx1 ^ 2 + vy1 ^ 2)
    magnitude2 = Sqr(vx2 ^ 2 + vy2 ^ 2)
    
    AngleBetweenVectors2D = Atn2(vy2, vx2) - Atn2(vy1, vx1)
    If AngleBetweenVectors2D < 0 Then AngleBetweenVectors2D = AngleBetweenVectors2D + 360
End Function

Public Function RotatePoint2D(x As Double, y As Double, angle As Double) As Variant
    Dim rad As Double
    rad = angle * (PI / 180)
    
    RotatePoint2D = Array(x * Cos(rad) - y * Sin(rad), x * Sin(rad) + y * Cos(rad))
End Function

Public Function Distance2D(x1 As Double, y1 As Double, x2 As Double, y2 As Double) As Double
    Distance2D = Sqr((x2 - x1) ^ 2 + (y2 - y1) ^ 2)
End Function

Public Function GravitationalForce(mass1 As Double, mass2 As Double, distance As Double) As Double
    Dim g As Double
    g = 0.000000000066743
    
    GravitationalForce = (g * mass1 * mass2) / (distance ^ 2)
End Function

Public Function Friction(force As Double, coefficient As Double) As Double
    Friction = force * coefficient
End Function

Public Function RotationMatrix2D(angle As Double) As Variant
    Dim rad As Double
    rad = angle * (PI / 180)
    
    RotationMatrix2D = Array(Array(Cos(rad), -Sin(rad)), Array(Sin(rad), Cos(rad)))
End Function

Public Function Determinant2x2(m As Variant) As Double
    Determinant2x2 = (m(0)(0) * m(1)(1)) - (m(0)(1) * m(1)(0))
End Function

Public Function MultiplyMatrix2D(m As Variant, v As Variant) As Variant
    MultiplyMatrix2D = Array(m(0)(0) * v(0) + m(0)(1) * v(1), m(1)(0) * v(0) + m(1)(1) * v(1))
End Function

Public Function CalculateBoundingBox2D(vertices As Variant) As Variant
    Dim minX As Double, minY As Double, maxX As Double, maxY As Double
    
    minX = vertices(0)(0)
    minY = vertices(0)(1)
    maxX = vertices(0)(0)
    maxY = vertices(0)(1)
    
    Dim i As Long
    For i = LBound(vertices) To UBound(vertices)
        If vertices(i)(0) < minX Then minX = vertices(i)(0)
        If vertices(i)(1) < minY Then minY = vertices(i)(1)
        If vertices(i)(0) > maxX Then maxX = vertices(i)(0)
        If vertices(i)(1) > maxY Then maxY = vertices(i)(1)
    Next i
    
    CalculateBoundingBox2D = Array(minX, minY, maxX, maxY)
End Function

Public Function EuclideanDistance(x1 As Double, y1 As Double, x2 As Double, y2 As Double) As Double
    EuclideanDistance = Sqr((x2 - x1) ^ 2 + (y2 - y1) ^ 2)
End Function

Public Function ManhattanDistance(x1 As Double, y1 As Double, x2 As Double, y2 As Double) As Double
    ManhattanDistance = Abs(x2 - x1) + Abs(y2 - y1)
End Function

Public Function ChebyshevDistance(x1 As Double, y1 As Double, x2 As Double, y2 As Double) As Double
    ChebyshevDistance = WorksheetFunction.Max(Abs(x2 - x1), Abs(y2 - y1))
End Function

Public Function DistanceBetweenPoints(x1 As Double, y1 As Double, x2 As Double, y2 As Double) As Double
    DistanceBetweenPoints = Sqr((x2 - x1) ^ 2 + (y2 - y1) ^ 2)
End Function

Public Function Lerp(a As Double, b As Double, t As Double) As Double
    Lerp = a + (b - a) * t
End Function

Public Function CubicLerp(a As Double, b As Double, t As Double) As Double
    CubicLerp = a + (b - a) * (t ^ 2 * (3 - 2 * t))
End Function

Public Function RotatePoint(x As Double, y As Double, cx As Double, cy As Double, angle As Double) As Variant
    Dim rad As Double
    Dim cosA As Double, sinA As Double
    Dim nx As Double, ny As Double
    
    rad = angle * (PI / 180)
    cosA = Cos(rad)
    sinA = Sin(rad)
    
    nx = cosA * (x - cx) - sinA * (y - cy) + cx
    ny = sinA * (x - cx) + cosA * (y - cy) + cy
    
    RotatePoint = Array(nx, ny)
End Function

Public Function Clamp(value As Double, Min As Double, Max As Double) As Double
    If value < Min Then
        Clamp = Min
    ElseIf value > Max Then
        Clamp = Max
    Else
        Clamp = value
    End If
End Function

Public Function RandomFloat(Min As Double, Max As Double) As Double
    RandomFloat = (Max - Min) * Rnd + Min
End Function

Public Function Map(value As Double, inMin As Double, inMax As Double, outMin As Double, outMax As Double) As Double
    Map = (value - inMin) / (inMax - inMin) * (outMax - outMin) + outMin
End Function

Public Function ProjectVector2D(vx As Double, vy As Double, ax As Double, ay As Double) As Variant
    Dim dotProd As Double
    Dim magSquared As Double
    
    dotProd = (vx * ax) + (vy * ay)
    magSquared = (ax ^ 2 + ay ^ 2)
    
    ProjectVector2D = Array((dotProd / magSquared) * ax, (dotProd / magSquared) * ay)
End Function

Public Function ReflectVector(vx As Double, vy As Double, vz As Double, nx As Double, ny As Double, nz As Double) As Variant
    Dim DotProduct As Double
    
    DotProduct = (vx * nx) + (vy * ny) + (vz * nz)
    ReflectVector = Array(vx - 2 * DotProduct * nx, vy - 2 * DotProduct * ny, vz - 2 * DotProduct * nz)
End Function

Public Function AngleBetweenLineAndPlane(x1 As Double, y1 As Double, z1 As Double, x2 As Double, y2 As Double, z2 As Double, nx As Double, ny As Double, nz As Double) As Double
    Dim DotProduct As Double
    Dim magnitudeLine As Double
    Dim magnitudeNormal As Double
    
    DotProduct = (x2 - x1) * nx + (y2 - y1) * ny + (z2 - z1) * nz
    magnitudeLine = Sqr((x2 - x1) ^ 2 + (y2 - y1) ^ 2 + (z2 - z1) ^ 2)
    magnitudeNormal = Sqr(nx ^ 2 + ny ^ 2 + nz ^ 2)
    AngleBetweenLineAndPlane = Acos(DotProduct / (magnitudeLine * magnitudeNormal)) * (180 / PI)
End Function

Public Function ParabolaPosition(v0 As Double, angle As Double, g As Double, t As Double) As Variant
    Dim x As Double, y As Double
    x = v0 * Cos(angle * (PI / 180)) * t
    y = v0 * Sin(angle * (PI / 180)) * t - (0.5 * g * t ^ 2)
    
    ParabolaPosition = Array(x, y)
End Function

Public Function InterpolateLinear(x1 As Double, y1 As Double, x2 As Double, y2 As Double, x As Double) As Double
    InterpolateLinear = y1 + (x - x1) * ((y2 - y1) / (x2 - x1))
End Function

Public Function VectorToMatrix(vx As Double, vy As Double, vz As Double) As Variant
    VectorToMatrix = Array(Array(vx), Array(vy), Array(vz))
End Function

Public Function CrossProductMagnitude(vx1 As Double, vy1 As Double, vz1 As Double, vx2 As Double, vy2 As Double, vz2 As Double) As Double
    Dim crossProduct As Variant
    crossProduct = CrossProduct3D(vx1, vy1, vz1, vx2, vy2, vz2)
    
    CrossProductMagnitude = Sqr(crossProduct(0) ^ 2 + crossProduct(1) ^ 2 + crossProduct(2) ^ 2)
End Function

Public Function CartesianToPolar(x As Double, y As Double) As Variant
    Dim r As Double
    Dim theta As Double
    r = Sqr(x ^ 2 + y ^ 2)
    theta = Atn2(y, x) * (180 / PI)
    
    CartesianToPolar = Array(r, theta)
End Function

Public Function PolarToCartesian(r As Double, theta As Double) As Variant
    Dim x As Double
    Dim y As Double
    x = r * Cos(theta * (PI / 180))
    y = r * Sin(theta * (PI / 180))
    
    PolarToCartesian = Array(x, y)
End Function

Public Function Factorial(n As Integer) As Double
    If n = 0 Or n = 1 Then
        Factorial = 1
    Else
        Factorial = n * Factorial(n - 1)
    End If
End Function

Public Function Power(base As Double, exponent As Double) As Double
    Power = base ^ exponent
End Function

Public Function Sqrt(x As Double) As Double
    Sqrt = x ^ 0.5
End Function

Public Function Mean(values As Variant) As Double
    Dim sum As Double
    Dim count As Long
    
    For count = LBound(values) To UBound(values)
        sum = sum + values(count)
    Next count
    
    Mean = sum / (UBound(values) - LBound(values) + 1)
End Function

Public Function Variance(values As Variant) As Double
    Dim meanValue As Double
    Dim sum As Double
    Dim count As Long
    
    meanValue = Mean(values)
    For count = LBound(values) To UBound(values)
        sum = sum + (values(count) - meanValue) ^ 2
    Next count
    
    Variance = sum / (UBound(values) - LBound(values) + 1)
End Function

Public Function StandardDeviation(values As Variant) As Double
    StandardDeviation = Sqr(Variance(values))
End Function

Public Function MeanVarianceStandardDeviation(values As Variant) As Variant
    Dim sum As Double, SumOfSquares As Double
    Dim count As Long
    
    For count = LBound(values) To UBound(values)
        sum = sum + values(count)
        SumOfSquares = SumOfSquares + values(count) ^ 2
    Next count
    
    Dim Mean As Double
    Mean = sum / (UBound(values) - LBound(values) + 1)
    
    Dim Variance As Double
    Variance = (SumOfSquares - (sum ^ 2) / (UBound(values) - LBound(values) + 1)) / (UBound(values) - LBound(values) + 1)
    
    Dim stdDev As Double
    stdDev = Sqr(Variance)
    
    MeanVarianceStandardDeviation = Array(Mean, Variance, stdDev)
End Function

Public Sub Inc(ByRef value As Double, Optional n As Double = 1#)
    value = value + n
End Sub

Public Sub Dec(ByRef value As Double, Optional n As Double = 1#)
    value = value - n
End Sub

Public Function Max(values As Variant) As Double
    Dim maxValue As Double
    Dim count As Long
    maxValue = values(LBound(values))
    
    For count = LBound(values) To UBound(values)
        If values(count) > maxValue Then maxValue = values(count)
    Next count
    
    Max = maxValue
End Function

Public Function Min(values As Variant) As Double
    Dim minValue As Double
    Dim count As Long
    minValue = values(LBound(values))
    
    For count = LBound(values) To UBound(values)
        If values(count) < minValue Then minValue = values(count)
    Next count
    
    Min = minValue
End Function

Public Function ArcSine(value As Double) As Double
    If Abs(value) <= 1 Then
        ArcSine = Atn(value / Sqr(1 - value ^ 2))
    End If
End Function

Public Function ArcCosine(value As Double) As Double
    If Abs(value) <= 1 Then
        ArcCosine = Atn(Sqr(1 - value ^ 2) / value)
    End If
End Function

Public Function ArcTangent(value As Double) As Double
    ArcTangent = Atn(value)
End Function

Public Function Secant(angle As Double) As Double
    Secant = 1 / Cos(angle)
End Function

Public Function Cosecant(angle As Double) As Double
    Cosecant = 1 / Sin(angle)
End Function

Public Function Cotangent(angle As Double) As Double
    Cotangent = 1 / Tan(angle)
End Function

Public Function Sinh(x As Double) As Double
    Sinh = (Exp(x) - Exp(-x)) / 2
End Function

Public Function Cosh(x As Double) As Double
    Cosh = (Exp(x) + Exp(-x)) / 2
End Function

Public Function Tanh(x As Double) As Double
    Tanh = Sinh(x) / Cosh(x)
End Function

Public Function ArcSinh(value As Double) As Double
    ArcSinh = Log(value + Sqr(value ^ 2 + 1))
End Function

Public Function ArcCosh(value As Double) As Double
    If value >= 1 Then
        ArcCosh = Log(value + Sqr(value ^ 2 - 1))
    Else
        ArcCosh = 0
    End If
End Function

Public Function ArcTanh(value As Double) As Double
    If Abs(value) < 1 Then
        ArcTanh = 0.5 * Log((1 + value) / (1 - value))
    Else
        ArcTanh = 0
    End If
End Function

Public Function ArithmeticProgression(a1 As Double, d As Double, n As Long) As Double
    ArithmeticProgression = a1 + (n - 1) * d
End Function

Public Function SumArithmeticProgression(a1 As Double, d As Double, n As Long) As Double
    SumArithmeticProgression = (n / 2) * (2 * a1 + (n - 1) * d)
End Function

Public Function GeometricProgression(a1 As Double, r As Double, n As Long) As Double
    GeometricProgression = a1 * (r ^ (n - 1))
End Function

Public Function SumGeometricProgression(a1 As Double, r As Double, n As Long) As Double
    If r <> 1 Then
        SumGeometricProgression = a1 * ((1 - r ^ n) / (1 - r))
    Else
        SumGeometricProgression = a1 * n
    End If
End Function

Public Function GeometricProgressionNegative(a1 As Double, r As Double, n As Long) As Double
    If r < 0 Then
        GeometricProgressionNegative = a1 * ((-r) ^ (n - 1))
    Else
        GeometricProgressionNegative = a1 * (r ^ (n - 1))
    End If
End Function

Public Function SumGeometricProgressionNegative(a1 As Double, r As Double, n As Long) As Double
    If r <> 1 Then
        SumGeometricProgressionNegative = a1 * ((1 - (-r) ^ n) / (1 - (-r)))
    Else
        SumGeometricProgressionNegative = a1 * n
    End If
End Function

Public Function QuadRoots(a As Double, b As Double, c As Double) As Variant
    Dim discriminant As Double
    discriminant = b ^ 2 - 4 * a * c
    
    If discriminant < 0 Then
        QuadRoots = Array()
    Else
        QuadRoots = Array((-b + Sqr(discriminant)) / (2 * a), (-b - Sqr(discriminant)) / (2 * a))
    End If
End Function

Public Function ArithmeticSeriesSum(a1 As Double, d As Double, n As Long) As Double
    ArithmeticSeriesSum = (n / 2) * (2 * a1 + (n - 1) * d)
End Function

Public Function GeometricSeriesSum(a1 As Double, r As Double, n As Long) As Double
    If r <> 1 Then
        GeometricSeriesSum = a1 * ((1 - r ^ n) / (1 - r))
    Else
        GeometricSeriesSum = a1 * n
    End If
End Function

Public Function ScalarMultiplyMatrix(matrix As Variant, scalar As Double) As Variant
    Dim row As Long, col As Long
    Dim result() As Double
    ReDim result(LBound(matrix) To UBound(matrix), LBound(matrix(0)) To UBound(matrix(0)))
    
    For row = LBound(matrix) To UBound(matrix)
        For col = LBound(matrix(0)) To UBound(matrix(0))
            result(row, col) = matrix(row, col) * scalar
        Next col
    Next row
    
    ScalarMultiplyMatrix = result
End Function

Public Function MatrixAdd(a As Variant, b As Variant) As Variant
    Dim row As Long, col As Long
    Dim result() As Double
    ReDim result(LBound(a) To UBound(a), LBound(a(0)) To UBound(a(0)))
    
    For row = LBound(a) To UBound(a)
        For col = LBound(a(0)) To UBound(a(0))
            result(row, col) = a(row, col) + b(row, col)
        Next col
    Next row
    
    MatrixAdd = result
End Function

Public Function WeightedMean(values As Variant, weights As Variant) As Double
    Dim sum As Double
    Dim weightSum As Double
    Dim i As Long
    
    For i = LBound(values) To UBound(values)
        sum = sum + values(i) * weights(i)
        weightSum = weightSum + weights(i)
    Next i
    
    WeightedMean = sum / weightSum
End Function

Public Function SumOfSquares(values As Variant) As Double
    Dim sum As Double
    Dim i As Long
    
    For i = LBound(values) To UBound(values)
        sum = sum + values(i) ^ 2
    Next i
    
    SumOfSquares = sum
End Function

Public Function CubeRoot(x As Double) As Double
    CubeRoot = x ^ (1 / 3)
End Function

Public Function InverseMatrix2x2(m As Variant) As Variant
    Dim det As Double
    det = Determinant2x2(m)
    
    If det = 0 Then
        InverseMatrix2x2 = Array()
    Else
        InverseMatrix2x2 = Array(Array(m(1)(1) / det, -m(0)(1) / det), Array(-m(1)(0) / det, m(0)(0) / det))
    End If
End Function

Public Function Exponential(x As Double) As Double
    Exponential = Exp(x)
End Function

Public Function NaturalLog(x As Double) As Double
    If x > 0 Then
        NaturalLog = Log(x)
    Else
        NaturalLog = 0
    End If
End Function

Public Function LogBase(x As Double, base As Double) As Double
    LogBase = Log(x) / Log(base)
End Function

Public Function Combination(n As Long, r As Long) As Double
    Combination = Factorial(n) / (Factorial(r) * Factorial(n - r))
End Function

Public Function Permutation(n As Long, r As Long) As Double
    Permutation = Factorial(n) / Factorial(n - r)
End Function

Public Function IsPrime(n As Long) As Boolean
    Dim i As Long
    
    If n < 2 Then
        IsPrime = False
    Else
        For i = 2 To Sqr(n)
            If n Mod i = 0 Then
                IsPrime = False
                Exit Function
            End If
        Next i
        IsPrime = True
    End If
End Function

Public Function AbsoluteError(actual As Double, expected As Double) As Double
    AbsoluteError = Abs(actual - expected)
End Function

Public Function RelativeError(actual As Double, expected As Double) As Double
    If expected <> 0 Then
        RelativeError = Abs((actual - expected) / expected)
    Else
        RelativeError = 0
    End If
End Function
