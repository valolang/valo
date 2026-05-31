Attribute VB_Name = "Riff"
'/**
' * Riff - Audio Engine (Studio DSP Edition)
' * @description A high-performance, COM-based WASAPI audio engine for VBA (x86/x64 compatible).
' * Contains advanced Array Chunking for zero-latency mixing, Polyphony, and a full
' * Studio DSP Pipeline featuring Freeverb-style Reverb, Chorus, Flanger, Compressor, Biquad EQ, Bitcrusher,
' * RingMod, AutoPan, Delay, BLEP Oscillators, In-Memory Loading, WAV Export, optimized decode v-table calls, Buses, and Peak Meters.
' * @author UesleiDev
' * @version 1.0.2
' */

Option Explicit
Option Private Module


#If VBA7 Then
    '/** @description Allocates physical memory pages. */
    Private Declare PtrSafe Function VirtualAlloc Lib "kernel32" (ByVal lpAddress As LongPtr, ByVal dwSize As LongPtr, ByVal flAllocationType As Long, ByVal flProtect As Long) As LongPtr
    
    '/** @description Frees allocated physical memory. */
    Private Declare PtrSafe Function VirtualFree Lib "kernel32" (ByVal lpAddress As LongPtr, ByVal dwSize As LongPtr, ByVal dwFreeType As Long) As Long
    
    '/** @description Fast memory block copy. */
    Private Declare PtrSafe Sub RtlMoveMemory Lib "kernel32" (ByVal Destination As LongPtr, ByVal Source As LongPtr, ByVal Length As LongPtr)
    
    '/** @description Memory copy overloaded for Single precision floats. */
    Private Declare PtrSafe Sub RtlMoveMemoryToSingle Lib "kernel32" Alias "RtlMoveMemory" (ByRef Destination As Single, ByVal Source As LongPtr, ByVal Length As LongPtr)
    
    '/** @description Memory copy overloaded for Integers. */
    Private Declare PtrSafe Sub RtlMoveMemoryToInteger Lib "kernel32" Alias "RtlMoveMemory" (ByRef Destination As Integer, ByVal Source As LongPtr, ByVal Length As LongPtr)
    
    '/** @description Clears a block of memory to zeros. */
    Private Declare PtrSafe Sub RtlZeroMemory Lib "kernel32" (ByVal Destination As LongPtr, ByVal Length As LongPtr)
    
    '/** @description Calls a window procedure (used for callbacks). */
    Private Declare PtrSafe Function CallWindowProcW Lib "user32" (ByVal lpPrevWndFunc As LongPtr, ByVal hWnd As LongPtr, ByVal Msg As Long, ByVal wParam As LongPtr, ByVal lParam As LongPtr) As LongPtr
    
    '/** @description Generic native pointer call shim for optimized x64 COM v-table calls. */
    Private Declare PtrSafe Function RiffCallPtr4 Lib "user32" Alias "CallWindowProcW" (ByVal lpPrevWndFunc As LongPtr, ByVal a0 As LongPtr, ByVal a1 As LongPtr, ByVal a2 As LongPtr, ByVal a3 As LongPtr) As LongPtr
    
    '/** @description Creates a COM object instance. */
    Private Declare PtrSafe Function CoCreateInstance Lib "ole32" (ByRef rclsid As Any, ByVal pUnkOuter As LongPtr, ByVal dwClsContext As Long, ByRef riid As Any, ByRef ppv As LongPtr) As Long
    
    '/** @description Initializes Media Foundation. */
    Private Declare PtrSafe Function MFStartup Lib "mfplat" (ByVal Version As Long, ByVal dwFlags As Long) As Long
    
    '/** @description Shuts down Media Foundation. */
    Private Declare PtrSafe Function MFShutdown Lib "mfplat" () As Long
    
    '/** @description Starts a multimedia timer for the engine loop. */
    Private Declare PtrSafe Function SetTimer Lib "user32" (ByVal hWnd As LongPtr, ByVal nIDEvent As LongPtr, ByVal uElapse As Long, ByVal lpTimerFunc As LongPtr) As LongPtr
    
    '/** @description Kills the multimedia timer. */
    Private Declare PtrSafe Function KillTimer Lib "user32" (ByVal hWnd As LongPtr, ByVal nIDEvent As LongPtr) As Long
    
    '/** @description Frees memory allocated by COM. */
    Private Declare PtrSafe Function CoTaskMemFree Lib "ole32" (ByVal pv As LongPtr) As Long
    
    '/** @description Low-level COM interface calling via VTable. */
    Private Declare PtrSafe Function DispCallFunc Lib "oleaut32" (ByVal pvInstance As LongPtr, ByVal oVft As LongPtr, ByVal cc As Long, ByVal vtReturn As Integer, ByVal cActuals As Long, ByRef prgvt As Any, ByRef prgpvarg As Any, ByRef pvargResult As Variant) As Long
    
    '/** @description Converts a String GUID to a structural GUID. */
    Private Declare PtrSafe Function IIDFromString Lib "ole32" (ByVal lpsz As LongPtr, ByRef lpiid As Any) As Long
    
    '/** @description Gets a module handle. */
    Private Declare PtrSafe Function GetModuleHandleA Lib "kernel32" (ByVal lpModuleName As String) As LongPtr
    
    '/** @description Gets a procedure address from a module. */
    Private Declare PtrSafe Function GetProcAddress Lib "kernel32" (ByVal hModule As LongPtr, ByVal lpProcName As String) As LongPtr
    
    '/** @description Gets a procedure address by ordinal. */
    Private Declare PtrSafe Function GetProcAddressOrdinal Lib "kernel32" Alias "GetProcAddress" (ByVal hModule As LongPtr, ByVal lpProcName As Long) As LongPtr
    
    '/** @description Creates an MF Source Reader from a file URL. */
    Private Declare PtrSafe Function MFCreateSourceReaderFromURL Lib "mfreadwrite" (ByVal pwszURL As LongPtr, ByVal pAttributes As LongPtr, ByRef ppSourceReader As LongPtr) As Long
    
    '/** @description Creates an empty MF Media Type. */
    Private Declare PtrSafe Function MFCreateMediaType Lib "mfplat" (ByRef ppMFType As LongPtr) As Long
    
    '/** @description Initializes MF Media Type from Wave Format. */
    Private Declare PtrSafe Function MFInitMediaTypeFromWaveFormatEx Lib "mfplat" (ByVal pMFType As LongPtr, ByVal pWaveFormat As LongPtr, ByVal cbBufSize As Long) As Long
    
    '/** @description Creates MF Attributes configuration store. */
    Private Declare PtrSafe Function MFCreateAttributes Lib "mfplat" (ByRef ppMFAttributes As LongPtr, ByVal cInitialSize As Long) As Long
    
    '/** @description Creates an IStream from a Byte Array in memory. */
    Private Declare PtrSafe Function SHCreateMemStream Lib "shlwapi.dll" (ByRef pInit As Any, ByVal cbInit As Long) As LongPtr
    
    '/** @description Wraps an IStream into an IMFByteStream. */
    Private Declare PtrSafe Function MFCreateMFByteStreamOnStream Lib "mfplat.dll" (ByVal pStream As LongPtr, ByRef ppByteStream As LongPtr) As Long
    
    '/** @description Creates an MF Source Reader directly from an IMFByteStream. */
    Private Declare PtrSafe Function MFCreateSourceReaderFromByteStream Lib "mfreadwrite.dll" (ByVal pByteStream As LongPtr, ByVal pAttributes As LongPtr, ByRef ppSourceReader As LongPtr) As Long
    
    '/** @description Increases system timer resolution. */
    Private Declare PtrSafe Function timeBeginPeriod Lib "winmm.dll" (ByVal uPeriod As Long) As Long
    
    '/** @description Restores system timer resolution. */
    Private Declare PtrSafe Function timeEndPeriod Lib "winmm.dll" (ByVal uPeriod As Long) As Long
    
    '/**
    ' * @struct RiffBuffer
    ' * @brief Holds physical memory pointers for static loaded PCM audio data.
    ' */
    Private Type RiffBuffer
        Active As Boolean
        BufferPtr As LongPtr
        BufferLen As Long
    End Type

    '/**
    ' * @struct RiffContext
    ' * @brief Global engine state, holding WASAPI and Media Foundation pointers.
    ' */
    Private Type RiffContext
        Initialized As Boolean
        MagicCookie As Long
        MasterVolume As Single
        MasterPeakL As Single
        MasterPeakR As Single
        sampleRate As Long
        AvgBytesPerSec As Long
        DeviceEnumerator As LongPtr
        Device As LongPtr
        AudioClient As LongPtr
        RenderClient As LongPtr
        MixFormatPtr As LongPtr
        BufferSize As Long
        ThunkTimerCB As LongPtr
        TimerID As LongPtr
        Buffers(0 To 63) As RiffBuffer
        Buses(0 To 7) As Single
    End Type
#Else
    '/** @description Allocates physical memory pages. */
    Private Declare Function VirtualAlloc Lib "kernel32" (ByVal lpAddress As Long, ByVal dwSize As Long, ByVal flAllocationType As Long, ByVal flProtect As Long) As Long
    
    '/** @description Frees allocated physical memory. */
    Private Declare Function VirtualFree Lib "kernel32" (ByVal lpAddress As Long, ByVal dwSize As Long, ByVal dwFreeType As Long) As Long
    
    '/** @description Fast memory block copy. */
    Private Declare Sub RtlMoveMemory Lib "kernel32" (ByVal Destination As Long, ByVal Source As Long, ByVal Length As Long)
    
    '/** @description Memory copy overloaded for Single precision floats. */
    Private Declare Sub RtlMoveMemoryToSingle Lib "kernel32" Alias "RtlMoveMemory" (ByRef Destination As Single, ByVal Source As Long, ByVal Length As Long)
    
    '/** @description Memory copy overloaded for Integers. */
    Private Declare Sub RtlMoveMemoryToInteger Lib "kernel32" Alias "RtlMoveMemory" (ByRef Destination As Integer, ByVal Source As Long, ByVal Length As Long)
    
    '/** @description Clears a block of memory to zeros. */
    Private Declare Sub RtlZeroMemory Lib "kernel32" (ByVal Destination As Long, ByVal Length As Long)
    
    '/** @description Calls a window procedure (used for callbacks). */
    Private Declare Function CallWindowProcW Lib "user32" (ByVal lpPrevWndFunc As Long, ByVal hWnd As Long, ByVal Msg As Long, ByVal wParam As Long, ByVal lParam As Long) As Long
    
    '/** @description Generic native pointer call shim for optimized COM v-table calls. */
    Private Declare Function RiffCallPtr4 Lib "user32" Alias "CallWindowProcW" (ByVal lpPrevWndFunc As Long, ByVal a0 As Long, ByVal a1 As Long, ByVal a2 As Long, ByVal a3 As Long) As Long
    
    '/** @description Creates a COM object instance. */
    Private Declare Function CoCreateInstance Lib "ole32" (ByRef rclsid As Any, ByVal pUnkOuter As Long, ByVal dwClsContext As Long, ByRef riid As Any, ByRef ppv As Long) As Long
    
    '/** @description Initializes Media Foundation. */
    Private Declare Function MFStartup Lib "mfplat" (ByVal Version As Long, ByVal dwFlags As Long) As Long
    
    '/** @description Shuts down Media Foundation. */
    Private Declare Function MFShutdown Lib "mfplat" () As Long
    
    '/** @description Starts a multimedia timer for the engine loop. */
    Private Declare Function SetTimer Lib "user32" (ByVal hWnd As Long, ByVal nIDEvent As Long, ByVal uElapse As Long, ByVal lpTimerFunc As Long) As Long
    
    '/** @description Kills the multimedia timer. */
    Private Declare Function KillTimer Lib "user32" (ByVal hWnd As Long, ByVal nIDEvent As Long) As Long
    
    '/** @description Frees memory allocated by COM. */
    Private Declare Function CoTaskMemFree Lib "ole32" (ByVal pv As Long) As Long
    
    '/** @description Low-level COM interface calling via VTable. */
    Private Declare Function DispCallFunc Lib "oleaut32" (ByVal pvInstance As Long, ByVal oVft As Long, ByVal cc As Long, ByVal vtReturn As Integer, ByVal cActuals As Long, ByRef prgvt As Any, ByRef prgpvarg As Any, ByRef pvargResult As Variant) As Long
    
    '/** @description Converts a String GUID to a structural GUID. */
    Private Declare Function IIDFromString Lib "ole32" (ByVal lpsz As Long, ByRef lpiid As Any) As Long
    
    '/** @description Gets a module handle. */
    Private Declare Function GetModuleHandleA Lib "kernel32" (ByVal lpModuleName As String) As Long
    
    '/** @description Gets a procedure address from a module. */
    Private Declare Function GetProcAddress Lib "kernel32" (ByVal hModule As Long, ByVal lpProcName As String) As Long
    
    '/** @description Gets a procedure address by ordinal. */
    Private Declare Function GetProcAddressOrdinal Lib "kernel32" Alias "GetProcAddress" (ByVal hModule As Long, ByVal lpProcName As Long) As Long
    
    '/** @description Creates an MF Source Reader from a file URL. */
    Private Declare Function MFCreateSourceReaderFromURL Lib "mfreadwrite" (ByVal pwszURL As Long, ByVal pAttributes As Long, ByRef ppSourceReader As Long) As Long
    
    '/** @description Creates an empty MF Media Type. */
    Private Declare Function MFCreateMediaType Lib "mfplat" (ByRef ppMFType As Long) As Long
    
    '/** @description Initializes MF Media Type from Wave Format. */
    Private Declare Function MFInitMediaTypeFromWaveFormatEx Lib "mfplat" (ByVal pMFType As Long, ByVal pWaveFormat As Long, ByVal cbBufSize As Long) As Long
    
    '/** @description Creates MF Attributes configuration store. */
    Private Declare Function MFCreateAttributes Lib "mfplat" (ByRef ppMFAttributes As Long, ByVal cInitialSize As Long) As Long
    
    '/** @description Creates an IStream from a Byte Array in memory. */
    Private Declare Function SHCreateMemStream Lib "shlwapi.dll" (ByRef pInit As Any, ByVal cbInit As Long) As Long
    
    '/** @description Wraps an IStream into an IMFByteStream. */
    Private Declare Function MFCreateMFByteStreamOnStream Lib "mfplat.dll" (ByVal pStream As Long, ByRef ppByteStream As Long) As Long
    
    '/** @description Creates an MF Source Reader directly from an IMFByteStream. */
    Private Declare Function MFCreateSourceReaderFromByteStream Lib "mfreadwrite.dll" (ByVal pByteStream As Long, ByVal pAttributes As Long, ByRef ppSourceReader As Long) As Long
    
    '/** @description Increases system timer resolution. */
    Private Declare Function timeBeginPeriod Lib "winmm.dll" (ByVal uPeriod As Long) As Long
    
    '/** @description Restores system timer resolution. */
    Private Declare Function timeEndPeriod Lib "winmm.dll" (ByVal uPeriod As Long) As Long
    
    '/**
    ' * @struct RiffBuffer
    ' * @brief Holds physical memory pointers for static loaded PCM audio data.
    ' */
    Private Type RiffBuffer
        Active As Boolean
        BufferPtr As Long
        BufferLen As Long
    End Type

    '/**
    ' * @struct RiffContext
    ' * @brief Global engine state, holding WASAPI and Media Foundation pointers.
    ' */
    Private Type RiffContext
        Initialized As Boolean
        MagicCookie As Long
        MasterVolume As Single
        MasterPeakL As Single
        MasterPeakR As Single
        sampleRate As Long
        AvgBytesPerSec As Long
        DeviceEnumerator As Long
        Device As Long
        AudioClient As Long
        RenderClient As Long
        MixFormatPtr As Long
        BufferSize As Long
        ThunkTimerCB As Long
        TimerID As Long
        Buffers(0 To 63) As RiffBuffer
        Buses(0 To 7) As Single
    End Type
#End If

'/**
' * @struct RiffVoice
' * @brief Polyphonic playback channel containing a full Studio DSP Pipeline matrix.
' */
Private Type RiffVoice
    Active As Boolean
    Playing As Boolean
    Paused As Boolean
    busID As Long
    BufferIndex As Long
    
    IsOscillator As Boolean
    OscType As Long
    OscFreq As Single
    OscPhase As Double
    
    PeakL As Single
    PeakR As Single
    
    Position As Double
    Volume As Single
    Pitch As Double
    Pan As Single
    
    AutoPanRate As Single
    AutoPanDepth As Single
    AutoPanPhase As Double
    
    BitcrushSteps As Single
    BitcrushDownsample As Long
    BitcrushDsCount As Long
    BitcrushLastL As Single
    BitcrushLastR As Single
    
    Distortion As Single
    
    EqBass As Single
    EqMid As Single
    EqTreble As Single
    EqStateLowL As Single
    EqStateLowR As Single
    EqStateHighL As Single
    EqStateHighR As Single
    
    lowPass As Single
    highPass As Single
    FilterStateL As Single
    FilterStateR As Single
    FilterStateHP_L As Single
    FilterStateHP_R As Single
    
    RingModFreq As Single
    RingModMix As Single
    RingModPhase As Double
    
    StereoWidth As Single
    
    TremoloRate As Single
    TremoloDepth As Single
    TremoloPhase As Double
    
    ChorusRate As Single
    ChorusDepth As Single
    ChorusPhase As Double
    
    FlangerRate As Single
    FlangerDepth As Single
    FlangerFeedback As Single
    FlangerPhase As Double
    
    ReverbMix As Single
    ReverbTime As Single
    RevTap1 As Long
    RevTap2 As Long
    RevTap3 As Long
    RevTap4 As Long
    RevDamp1L As Single
    RevDamp1R As Single
    RevDamp2L As Single
    RevDamp2R As Single
    RevDamp3L As Single
    RevDamp3R As Single
    RevDamp4L As Single
    RevDamp4R As Single
    
    BqLowPassZ1L As Single
    BqLowPassZ2L As Single
    BqLowPassZ1R As Single
    BqLowPassZ2R As Single
    BqHighPassZ1L As Single
    BqHighPassZ2L As Single
    BqHighPassZ1R As Single
    BqHighPassZ2R As Single
    EqBassZ1L As Single
    EqBassZ2L As Single
    EqBassZ1R As Single
    EqBassZ2R As Single
    EqMidZ1L As Single
    EqMidZ2L As Single
    EqMidZ1R As Single
    EqMidZ2R As Single
    EqTrebleZ1L As Single
    EqTrebleZ2L As Single
    EqTrebleZ1R As Single
    EqTrebleZ2R As Single
    
    DelayTime As Single
    DelayFeedback As Single
    DelayMix As Single
    
    CompThreshold As Single
    CompRatio As Single
    CompEnv As Single
    
    RingWritePos As Long
    
    Looping As Boolean
    loopStart As Double
    loopEnd As Double
    
    fadeState As Long
    FadeFramesTotal As Long
    FadeFramesCurrent As Long
End Type

'/**
' * @struct GUID
' * @brief Standard Windows COM GUID structure.
' */
Private Type GUID
    Data1 As Long
    Data2 As Integer
    Data3 As Integer
    Data4(0 To 7) As Byte
End Type

'/** @description Memory allocation flag for VirtualAlloc. */
Private Const MEM_COMMIT As Long = &H1000

'/** @description Memory reservation flag for VirtualAlloc. */
Private Const MEM_RESERVE As Long = &H2000

'/** @description Memory release flag for VirtualFree. */
Private Const MEM_RELEASE As Long = &H8000

'/** @description Read/Write protection flag for Memory. */
Private Const PAGE_READWRITE As Long = &H4

'/** @description Execute/Read/Write protection flag for Thunk Callbacks. */
Private Const PAGE_EXECUTE_READWRITE As Long = &H40

'/** @description Context flag for COM instantiation. */
Private Const CLSCTX_ALL As Long = 23

'/** @description Media Foundation Version constant. */
Private Const MF_VERSION As Long = &H20070

'/** @description Calling convention standard for COM. */
Private Const CC_STDCALL As Long = 4

'/** @description WASAPI Shared Mode constant. */
Private Const AUDCLNT_SHAREMODE_SHARED As Long = 0

'/** @description WASAPI Render endpoint. */
Private Const eRender As Long = 0

'/** @description WASAPI Console role. */
Private Const eConsole As Long = 0

'/** @description Media Foundation stream selection flag. */
Private Const MF_SOURCE_READER_ALL_STREAMS As Long = -2&

'/** @description Media Foundation first audio stream flag. */
Private Const MF_SOURCE_READER_FIRST_AUDIO_STREAM As Long = -3&

'/** @description Mathematical constant Pi. */
Private Const PI As Double = 3.14159265358979

'/** @description Mathematical constant 2 * Pi. */
Private Const PI2 As Double = 6.28318530717958

'/** @description Global state holding hardware info and context. */
Private rCtx As RiffContext

'/** @description Pool of 32 polyphonic voices for audio playback. */
Private rVoices(0 To 31) As RiffVoice

'/**
' * @description Contiguous global ring buffer array for spatial effects.
' * Solves the Column-Major 2D wipe issue by providing 1D sequential access.
' */
Private rRingBuf() As Single


'/**
' * @function RiffOpen
' * @brief Initializes the WASAPI Audio Engine, Media Foundation, and background Timers.
' * @return {Boolean} True if initialization was successful.
' */
Public Function RiffOpen() As Boolean
    If rCtx.Initialized Then
        RiffOpen = True
        Exit Function
    End If

    rCtx.MagicCookie = &H52494646
    rCtx.MasterVolume = 1!
    rCtx.MasterPeakL = 0!
    rCtx.MasterPeakR = 0!

    Dim i As Long
    For i = 0 To 7
        rCtx.Buses(i) = 1!
    Next i

    If Not InitThunks() Then
        Exit Function
    End If

    If MFStartup(MF_VERSION, 0) <> 0 Then
        FreeThunks
        Exit Function
    End If

    If Not InitWASAPI() Then
        ReleaseWASAPI
        MFShutdown
        FreeThunks
        Exit Function
    End If

    ReDim rRingBuf(0 To (32 * 192000) + 1)

    timeBeginPeriod 1

    rCtx.TimerID = SetTimer(0, 0, 15, rCtx.ThunkTimerCB)

    If rCtx.TimerID = 0 Then
        timeEndPeriod 1
        ReleaseWASAPI
        MFShutdown
        FreeThunks
        Exit Function
    End If

    rCtx.Initialized = True
    RiffOpen = True
End Function

'/**
' * @function RiffClose
' * @brief Safely shuts down the audio engine, frees memory, and releases COM objects.
' */
Public Sub RiffClose()
    If Not rCtx.Initialized Then
        Exit Sub
    End If
    
    rCtx.MagicCookie = 0
    
    If rCtx.TimerID <> 0 Then
        KillTimer 0, rCtx.TimerID
        rCtx.TimerID = 0
        timeEndPeriod 1
    End If
    
    Dim i As Long
    For i = 0 To 31
        rVoices(i).Active = False
    Next i
    
    For i = 0 To 63
        If rCtx.Buffers(i).Active Then
            RiffUnload i
        End If
    Next i
    
    Erase rRingBuf
    
    ReleaseWASAPI
    MFShutdown
    FreeThunks
    
    rCtx.Initialized = False
End Sub


'/**
' * @property RiffIsInitialized
' * @brief Returns True if the audio engine is running.
' */
Public Property Get RiffIsInitialized() As Boolean
    RiffIsInitialized = rCtx.Initialized
End Property

'/**
' * @property RiffMasterVolume
' * @brief Global master volume for the engine (0.0 to 1.0).
' */
Public Property Get RiffMasterVolume() As Single
    If Not rCtx.Initialized Then
        Exit Property
    End If
    
    RiffMasterVolume = rCtx.MasterVolume
End Property
Public Property Let RiffMasterVolume(ByVal value As Single)
    If Not rCtx.Initialized Then
        Exit Property
    End If
    
    If value < 0! Then
        value = 0!
    End If
    If value > 1! Then
        value = 1!
    End If
    
    rCtx.MasterVolume = value
End Property

'/**
' * @property RiffBusVolume
' * @brief Sets the volume multiplier for a specific Audio Bus (0 to 7).
' * @param busID The target bus index.
' */
Public Property Get RiffBusVolume(ByVal busID As Long) As Single
    If Not rCtx.Initialized Then
        Exit Property
    End If
    
    If busID < 0 Then
        busID = 0
    End If
    If busID > 7 Then
        busID = 7
    End If
    
    RiffBusVolume = rCtx.Buses(busID)
End Property
Public Property Let RiffBusVolume(ByVal busID As Long, ByVal value As Single)
    If Not rCtx.Initialized Then
        Exit Property
    End If
    
    If busID < 0 Then
        busID = 0
    End If
    If busID > 7 Then
        busID = 7
    End If
    
    If value < 0! Then
        value = 0!
    End If
    If value > 2! Then
        value = 2!
    End If
    
    rCtx.Buses(busID) = value
End Property

'/**
' * @function RiffMasterGetPeak
' * @brief Retrieves the absolute peak amplitude of the master output for VU Meters.
' * @param peakLeft Variable to store the left channel peak.
' * @param peakRight Variable to store the right channel peak.
' */
Public Sub RiffMasterGetPeak(ByRef peakLeft As Single, ByRef peakRight As Single)
    If Not rCtx.Initialized Then
        peakLeft = 0!
        peakRight = 0!
        Exit Sub
    End If
    
    peakLeft = rCtx.MasterPeakL
    peakRight = rCtx.MasterPeakR
End Sub

'/**
' * @function RiffLoad
' * @brief Decodes an audio file from disk into memory for zero-latency playback.
' * @param filePath Full path to the audio file (WAV, MP3, etc.).
' * @return {Long} Buffer handle (0-63), or -1 if failed.
' */
Public Function RiffLoad(ByVal filePath As String) As Long
    RiffLoad = -1
    
    If Not rCtx.Initialized Then
        Exit Function
    End If
    
    Dim slot As Long
    Dim i As Long
    slot = -1
    
    For i = 0 To 63
        If Not rCtx.Buffers(i).Active Then
            slot = i
            Exit For
        End If
    Next i
    
    If slot = -1 Then
        Exit Function
    End If
    
    If Dir(filePath) = "" Then
        Exit Function
    End If
    
    #If VBA7 Then
        Dim pReader As LongPtr
        Dim pPartialType As LongPtr
        Dim pAttributes As LongPtr
        Dim pNullPtr As LongPtr
        pNullPtr = 0
    #Else
        Dim pReader As Long
        Dim pPartialType As Long
        Dim pAttributes As Long
        Dim pNullPtr As Long
        pNullPtr = 0
    #End If
    
    Dim hr As Long
    Dim guidResample As GUID
    IIDFromString StrPtr("{7632CB14-D379-4770-AE7D-EA24154D9298}"), guidResample
    
    hr = MFCreateAttributes(pAttributes, 1)
    
    If hr = 0 And pAttributes <> 0 Then
        vCall pAttributes, 21, VarPtr(guidResample), 1&
        hr = MFCreateSourceReaderFromURL(StrPtr(filePath), pAttributes, pReader)
        vCall pAttributes, 2
    Else
        hr = MFCreateSourceReaderFromURL(StrPtr(filePath), 0, pReader)
    End If
    
    If hr <> 0 Or pReader = 0 Then
        Exit Function
    End If
    
    RiffLoad = CoreProcessSourceReader(pReader, slot)
End Function

'/**
' * @function RiffLoadFromMemory
' * @brief Decodes audio data directly from a Byte Array without requiring disk I/O.
' * @param audioData Byte array containing the binary audio file.
' * @return {Long} Buffer handle (0-63), or -1 if failed.
' */
Public Function RiffLoadFromMemory(ByRef audioData() As Byte) As Long
    RiffLoadFromMemory = -1
    
    If Not rCtx.Initialized Then
        Exit Function
    End If
    
    Dim slot As Long
    Dim i As Long
    slot = -1
    
    For i = 0 To 63
        If Not rCtx.Buffers(i).Active Then
            slot = i
            Exit For
        End If
    Next i
    
    If slot = -1 Then
        Exit Function
    End If
    
    #If VBA7 Then
        Dim pStream As LongPtr
        Dim pByteStream As LongPtr
        Dim pReader As LongPtr
        Dim pAttributes As LongPtr
    #Else
        Dim pStream As Long
        Dim pByteStream As Long
        Dim pReader As Long
        Dim pAttributes As Long
    #End If
    
    Dim cbSize As Long
    On Error Resume Next
    cbSize = UBound(audioData) - LBound(audioData) + 1
    If Err.Number <> 0 Then
        Err.Clear
        Exit Function
    End If
    On Error GoTo 0
    
    If cbSize <= 0 Then
        Exit Function
    End If
    
    pStream = SHCreateMemStream(audioData(LBound(audioData)), cbSize)
    If pStream = 0 Then
        Exit Function
    End If
    
    Dim hr As Long
    hr = MFCreateMFByteStreamOnStream(pStream, pByteStream)
    
    If hr <> 0 Or pByteStream = 0 Then
        vCall pStream, 2
        Exit Function
    End If
    
    Dim guidResample As GUID
    IIDFromString StrPtr("{7632CB14-D379-4770-AE7D-EA24154D9298}"), guidResample
    
    hr = MFCreateAttributes(pAttributes, 1)
    
    If hr = 0 And pAttributes <> 0 Then
        vCall pAttributes, 21, VarPtr(guidResample), 1&
        hr = MFCreateSourceReaderFromByteStream(pByteStream, pAttributes, pReader)
        vCall pAttributes, 2
    Else
        hr = MFCreateSourceReaderFromByteStream(pByteStream, 0, pReader)
    End If
    
    If hr = 0 And pReader <> 0 Then
        RiffLoadFromMemory = CoreProcessSourceReader(pReader, slot)
    End If
    
    If pByteStream <> 0 Then
        vCall pByteStream, 2
    End If
    
    If pStream <> 0 Then
        vCall pStream, 2
    End If
End Function

'/**
' * @function CoreProcessSourceReader
' * @brief Private helper to extract uncompressed PCM data from an IMFSourceReader into memory.
' * @param pReader Pointer to the IMFSourceReader.
' * @param slot Target buffer index to store the extracted audio.
' * @return {Long} Buffer handle if successful, or -1 if failed.
' */
#If VBA7 Then
Private Function CoreProcessSourceReader(ByVal pReader As LongPtr, ByVal slot As Long) As Long
#Else
Private Function CoreProcessSourceReader(ByVal pReader As Long, ByVal slot As Long) As Long
#End If
    CoreProcessSourceReader = -1
    Dim hr As Long
    
    #If VBA7 Then
        Dim pPartialType As LongPtr
        Dim pSample As LongPtr
        Dim pBuffer As LongPtr
        Dim pAudioData As LongPtr
        Dim pNullPtr As LongPtr
        Dim tempPtr As LongPtr
        Dim newPtr As LongPtr
        #If Win64 Then
            Dim llTime As LongLong
            pNullPtr = CLngLng(0)
        #Else
            Dim llTime As Currency
            pNullPtr = CLngPtr(0)
        #End If
    #Else
        Dim pPartialType As Long
        Dim pSample As Long
        Dim pBuffer As Long
        Dim pAudioData As Long
        Dim pNullPtr As Long
        Dim tempPtr As Long
        Dim newPtr As Long
        Dim llTime As Currency
        pNullPtr = 0
    #End If
    
    vCall pReader, 4, MF_SOURCE_READER_ALL_STREAMS, 0&
    vCall pReader, 4, MF_SOURCE_READER_FIRST_AUDIO_STREAM, 1&
    
    hr = MFCreateMediaType(pPartialType)
    If hr = 0 And pPartialType <> 0 Then
        Dim wfx_cbSize As Integer
        RtlMoveMemory VarPtr(wfx_cbSize), ByVal (rCtx.MixFormatPtr + 16), 2
        
        hr = MFInitMediaTypeFromWaveFormatEx(pPartialType, rCtx.MixFormatPtr, 18 + CLng(wfx_cbSize))
        If hr = 0 Then
            vCall pReader, 7, MF_SOURCE_READER_FIRST_AUDIO_STREAM, pNullPtr, pPartialType
        End If
        vCall pPartialType, 2
    End If
    
    Dim pSz As Long
    #If Win64 Then
        pSz = 8
    #Else
        pSz = 4
    #End If

    Dim dwFlags As Long
    Dim cbMax As Long
    Dim cbLen As Long
    
    Dim rArgs(0 To 5) As Variant
    Dim rTypes(0 To 5) As Integer
    Dim cArgs(0) As Variant
    Dim cTypes(0) As Integer
    Dim lArgs(0 To 2) As Variant
    Dim lTypes(0 To 2) As Integer
    Dim vRet As Variant
    Dim hrInvoke As Long
    Dim i As Long
    
    #If VBA7 Then
        Dim rPtrs(0 To 5) As LongPtr
        Dim cPtrs(0) As LongPtr
        Dim lPtrs(0 To 2) As LongPtr
    #Else
        Dim rPtrs(0 To 5) As Long
        Dim cPtrs(0) As Long
        Dim lPtrs(0 To 2) As Long
    #End If

    rArgs(0) = CLng(MF_SOURCE_READER_FIRST_AUDIO_STREAM)
    rArgs(1) = CLng(0)
    rArgs(2) = pNullPtr
    rArgs(3) = VarPtr(dwFlags)
    rArgs(4) = VarPtr(llTime)
    rArgs(5) = VarPtr(pSample)
    
    For i = 0 To 5
        rTypes(i) = VarType(rArgs(i))
        rPtrs(i) = VarPtr(rArgs(i))
    Next i

    cArgs(0) = VarPtr(pBuffer)
    cTypes(0) = VarType(cArgs(0))
    cPtrs(0) = VarPtr(cArgs(0))

    lArgs(0) = VarPtr(pAudioData)
    lArgs(1) = VarPtr(cbMax)
    lArgs(2) = VarPtr(cbLen)
    
    For i = 0 To 2
        lTypes(i) = VarType(lArgs(i))
        lPtrs(i) = VarPtr(lArgs(i))
    Next i

    Dim totalSize As Long
    Dim currentCap As Long
    
    currentCap = 1048576
    tempPtr = VirtualAlloc(0, currentCap, MEM_COMMIT Or MEM_RESERVE, PAGE_READWRITE)
    
    If tempPtr = 0 Then
        vCall pReader, 2
        Exit Function
    End If
    
    totalSize = 0
    Do
        pSample = 0
        hrInvoke = DispCallFunc(pReader, 9 * pSz, CC_STDCALL, vbLong, 6, rTypes(0), rPtrs(0), vRet)
        
        If hrInvoke <> 0 Then
            hr = hrInvoke
        Else
            hr = CLng(vRet)
        End If
        
        If hr <> 0 Or (dwFlags And 2) <> 0 Then
            Exit Do
        End If
        
        If pSample <> 0 Then
            hr = vCall(pSample, 41, VarPtr(pBuffer))
            
            If hr = 0 And pBuffer <> 0 Then
                hr = vCall(pBuffer, 3, VarPtr(pAudioData), VarPtr(cbMax), VarPtr(cbLen))
                
                If hr = 0 And pAudioData <> 0 And cbLen > 0 Then
                    If totalSize + cbLen > currentCap Then
                        currentCap = (totalSize + cbLen) * 2
                        newPtr = VirtualAlloc(0, currentCap, MEM_COMMIT Or MEM_RESERVE, PAGE_READWRITE)
                        
                        If newPtr <> 0 Then
                            RtlMoveMemory ByVal newPtr, ByVal tempPtr, totalSize
                            VirtualFree tempPtr, 0, MEM_RELEASE
                            tempPtr = newPtr
                        End If
                    End If
                    
                    If tempPtr <> 0 Then
                        RtlMoveMemory ByVal (tempPtr + totalSize), ByVal pAudioData, cbLen
                        totalSize = totalSize + cbLen
                    End If
                    
                    vCall pBuffer, 4
                End If
            End If
            
            If pBuffer <> 0 Then
                vCall pBuffer, 2
            End If
            
            vCall pSample, 2
        End If
    Loop
    
    vCall pReader, 2
    
    If totalSize > 0 Then
        rCtx.Buffers(slot).BufferPtr = VirtualAlloc(0, totalSize, MEM_COMMIT Or MEM_RESERVE, PAGE_READWRITE)
        
        If rCtx.Buffers(slot).BufferPtr <> 0 Then
            RtlMoveMemory ByVal rCtx.Buffers(slot).BufferPtr, ByVal tempPtr, totalSize
            rCtx.Buffers(slot).BufferLen = totalSize
            rCtx.Buffers(slot).Active = True
            CoreProcessSourceReader = slot
        End If
    End If
    
    If tempPtr <> 0 Then
        VirtualFree tempPtr, 0, MEM_RELEASE
    End If
End Function

'/**
' * @function RiffUnload
' * @brief Removes a static audio buffer from memory and stops associated playing voices.
' * @param bufferHandle The handle of the buffer to release.
' */
Public Sub RiffUnload(ByVal bufferHandle As Long)
    If Not rCtx.Initialized Then
        Exit Sub
    End If
    If bufferHandle < 0 Or bufferHandle > 63 Then
        Exit Sub
    End If
    If Not rCtx.Buffers(bufferHandle).Active Then
        Exit Sub
    End If
    
    Dim i As Long
    For i = 0 To 31
        If rVoices(i).Active And rVoices(i).BufferIndex = bufferHandle Then
            rVoices(i).Active = False
        End If
    Next i
    
    If rCtx.Buffers(bufferHandle).BufferPtr <> 0 Then
        VirtualFree rCtx.Buffers(bufferHandle).BufferPtr, 0, MEM_RELEASE
        rCtx.Buffers(bufferHandle).BufferPtr = 0
    End If
    
    rCtx.Buffers(bufferHandle).Active = False
End Sub

'/**
' * @property RiffBufferDurationSec
' * @brief Calculates the total duration of a statically loaded buffer in seconds.
' * @param bufferHandle The target buffer.
' */
Public Property Get RiffBufferDurationSec(ByVal bufferHandle As Long) As Single
    If Not rCtx.Initialized Or bufferHandle < 0 Or bufferHandle > 63 Then
        Exit Property
    End If
    If Not rCtx.Buffers(bufferHandle).Active Or rCtx.AvgBytesPerSec = 0 Then
        Exit Property
    End If
    
    RiffBufferDurationSec = CSng(rCtx.Buffers(bufferHandle).BufferLen) / CSng(rCtx.AvgBytesPerSec)

End Property

'/**
' * @function RiffExportBufferWav
' * @brief Exports a loaded buffer as a standard 16-bit stereo PCM WAV file.
' * @param bufferHandle Loaded buffer handle.
' * @param filePath Target WAV path.
' * @return {Boolean} True when the file was written successfully.
' */
Public Function RiffExportBufferWav(ByVal bufferHandle As Long, ByVal filePath As String) As Boolean
    If Not rCtx.Initialized Then
        Exit Function
    End If
    If bufferHandle < 0 Or bufferHandle > 63 Then
        Exit Function
    End If
    If Not rCtx.Buffers(bufferHandle).Active Then
        Exit Function
    End If
    If LenB(filePath) = 0 Then
        Exit Function
    End If

    Dim nChannels As Integer
    Dim nBlockAlign As Integer
    Dim wBits As Integer
    Dim sampleRate As Long
    Dim frames As Long
    Dim dataBytes As Long
    Dim outBytes() As Byte
    Dim frame As Long
    Dim outIndex As Long
    Dim sL As Single
    Dim sR As Single
    Dim iL As Integer
    Dim iR As Integer
    Dim isFloat As Boolean

    RtlMoveMemory VarPtr(nChannels), ByVal (rCtx.MixFormatPtr + 2), 2
    RtlMoveMemory VarPtr(sampleRate), ByVal (rCtx.MixFormatPtr + 4), 4
    RtlMoveMemory VarPtr(nBlockAlign), ByVal (rCtx.MixFormatPtr + 12), 2
    RtlMoveMemory VarPtr(wBits), ByVal (rCtx.MixFormatPtr + 14), 2

    If nChannels <= 0 Or nBlockAlign <= 0 Or sampleRate <= 0 Then
        Exit Function
    End If

    frames = rCtx.Buffers(bufferHandle).BufferLen \ CLng(nBlockAlign)
    If frames <= 0 Then
        Exit Function
    End If

    isFloat = RiffMixFormatIsFloat32()
    dataBytes = frames * 4
    ReDim outBytes(0 To dataBytes - 1)

    For frame = 0 To frames - 1
        sL = RiffReadInterleavedSample(rCtx.Buffers(bufferHandle).BufferPtr, frame, 0, nChannels, nBlockAlign, wBits, isFloat)
        If nChannels > 1 Then
            sR = RiffReadInterleavedSample(rCtx.Buffers(bufferHandle).BufferPtr, frame, 1, nChannels, nBlockAlign, wBits, isFloat)
        Else
            sR = sL
        End If

        iL = RiffFloatToPcm16(sL)
        iR = RiffFloatToPcm16(sR)
        outIndex = frame * 4
        RtlMoveMemory VarPtr(outBytes(outIndex)), VarPtr(iL), 2
        RtlMoveMemory VarPtr(outBytes(outIndex + 2)), VarPtr(iR), 2
    Next frame

    RiffExportBufferWav = RiffWritePcm16StereoWav(filePath, sampleRate, outBytes)
End Function

'/**
' * @function RiffRenderOscillatorWav
' * @brief Renders a band-limited oscillator directly to a 16-bit stereo PCM WAV file.
' * @param waveType 0=Sine, 1=Square, 2=Sawtooth, 3=Noise.
' * @param frequencyHz Oscillator frequency in Hz.
' * @param durationSec Render duration in seconds.
' * @param filePath Target WAV path.
' * @return {Boolean} True when the file was written successfully.
' */
Public Function RiffRenderOscillatorWav(ByVal waveType As Long, ByVal frequencyHz As Single, ByVal durationSec As Single, ByVal filePath As String) As Boolean
    If Not rCtx.Initialized Then
        Exit Function
    End If
    If durationSec <= 0! Then
        Exit Function
    End If
    If frequencyHz < 1! Then
        frequencyHz = 440!
    End If
    If waveType < 0 Then
        waveType = 0
    End If
    If waveType > 3 Then
        waveType = 3
    End If

    Dim frames As Long
    Dim outBytes() As Byte
    Dim frame As Long
    Dim outIndex As Long
    Dim phase As Double
    Dim dt As Double
    Dim sample As Single
    Dim pcm As Integer

    frames = CLng(durationSec * rCtx.sampleRate)
    If frames <= 0 Then
        Exit Function
    End If

    ReDim outBytes(0 To (frames * 4) - 1)
    dt = CDbl(frequencyHz) / CDbl(rCtx.sampleRate)

    For frame = 0 To frames - 1
        sample = RiffOscillatorSampleAtPhase(waveType, phase, dt)
        pcm = RiffFloatToPcm16(sample * 0.75!)
        outIndex = frame * 4
        RtlMoveMemory VarPtr(outBytes(outIndex)), VarPtr(pcm), 2
        RtlMoveMemory VarPtr(outBytes(outIndex + 2)), VarPtr(pcm), 2
        phase = phase + dt
        If phase >= 1# Then
            phase = phase - Int(phase)
        End If
    Next frame

    RiffRenderOscillatorWav = RiffWritePcm16StereoWav(filePath, rCtx.sampleRate, outBytes)
End Function

'/**
' * @function RiffFloatToPcm16
' * @brief Converts a normalized float sample to signed 16-bit PCM.
' */
Private Function RiffFloatToPcm16(ByVal value As Single) As Integer
    If value > 1! Then
        value = 1!
    ElseIf value < -1! Then
        value = -1!
    End If

    If value >= 0! Then
        RiffFloatToPcm16 = CInt(value * 32767!)
    Else
        RiffFloatToPcm16 = CInt(value * 32768!)
    End If
End Function

'/**
' * @function RiffReadInterleavedSample
' * @brief Reads a sample from an interleaved PCM/float buffer and normalizes it to -1.0..1.0.
' */
#If VBA7 Then
Private Function RiffReadInterleavedSample(ByVal basePtr As LongPtr, ByVal frameIndex As Long, ByVal channelIndex As Long, ByVal nChannels As Integer, ByVal nBlockAlign As Integer, ByVal wBits As Integer, ByVal isFloat As Boolean) As Single
#Else
Private Function RiffReadInterleavedSample(ByVal basePtr As Long, ByVal frameIndex As Long, ByVal channelIndex As Long, ByVal nChannels As Integer, ByVal nBlockAlign As Integer, ByVal wBits As Integer, ByVal isFloat As Boolean) As Single
#End If
    #If VBA7 Then
        Dim pSample As LongPtr
    #Else
        Dim pSample As Long
    #End If

    pSample = basePtr + (CLng(frameIndex) * CLng(nBlockAlign)) + ((CLng(channelIndex) Mod CLng(nChannels)) * (CLng(wBits) \ 8))

    If wBits = 32 And isFloat Then
        Dim f As Single
        RtlMoveMemory VarPtr(f), ByVal pSample, 4
        RiffReadInterleavedSample = f
    ElseIf wBits = 32 Then
        Dim l As Long
        RtlMoveMemory VarPtr(l), ByVal pSample, 4
        RiffReadInterleavedSample = CSng(CDbl(l) / 2147483648#)
    ElseIf wBits = 24 Then
        Dim b0 As Byte
        Dim b1 As Byte
        Dim b2 As Byte
        Dim v As Long
        RtlMoveMemory VarPtr(b0), ByVal pSample, 1
        RtlMoveMemory VarPtr(b1), ByVal (pSample + 1), 1
        RtlMoveMemory VarPtr(b2), ByVal (pSample + 2), 1
        v = CLng(b0) Or (CLng(b1) * &H100&) Or (CLng(b2) * &H10000)
        If (b2 And &H80) <> 0 Then
            v = v Or -16777216
        End If
        RiffReadInterleavedSample = CSng(CDbl(v) / 8388608#)
    ElseIf wBits = 16 Then
        Dim i As Integer
        RtlMoveMemory VarPtr(i), ByVal pSample, 2
        RiffReadInterleavedSample = CSng(CDbl(i) / 32768#)
    End If
End Function

'/**
' * @function RiffWritePcm16StereoWav
' * @brief Writes a 16-bit stereo PCM WAV file from an interleaved byte buffer.
' */
Private Function RiffWritePcm16StereoWav(ByVal filePath As String, ByVal sampleRate As Long, ByRef dataBytes() As Byte) As Boolean
    On Error GoTo Fail

    Dim f As Integer
    Dim dataSize As Long
    Dim riffSize As Long
    Dim byteRate As Long
    Dim blockAlign As Integer
    Dim bits As Integer
    Dim channels As Integer
    Dim fmtSize As Long
    Dim audioFormat As Integer

    dataSize = UBound(dataBytes) - LBound(dataBytes) + 1
    riffSize = 36 + dataSize
    channels = 2
    bits = 16
    blockAlign = 4
    byteRate = sampleRate * CLng(blockAlign)
    fmtSize = 16
    audioFormat = 1

    f = FreeFile
    Open filePath For Binary Access Write As #f
    Put #f, , CByte(Asc("R"))
    Put #f, , CByte(Asc("I"))
    Put #f, , CByte(Asc("F"))
    Put #f, , CByte(Asc("F"))
    Put #f, , riffSize
    Put #f, , CByte(Asc("W"))
    Put #f, , CByte(Asc("A"))
    Put #f, , CByte(Asc("V"))
    Put #f, , CByte(Asc("E"))
    Put #f, , CByte(Asc("f"))
    Put #f, , CByte(Asc("m"))
    Put #f, , CByte(Asc("t"))
    Put #f, , CByte(Asc(" "))
    Put #f, , fmtSize
    Put #f, , audioFormat
    Put #f, , channels
    Put #f, , sampleRate
    Put #f, , byteRate
    Put #f, , blockAlign
    Put #f, , bits
    Put #f, , CByte(Asc("d"))
    Put #f, , CByte(Asc("a"))
    Put #f, , CByte(Asc("t"))
    Put #f, , CByte(Asc("a"))
    Put #f, , dataSize
    Put #f, , dataBytes
    Close #f

    RiffWritePcm16StereoWav = True
    Exit Function

Fail:
    On Error Resume Next
    If f <> 0 Then
        Close #f
    End If
End Function

'/**
' * @function RiffOscillatorSampleAtPhase
' * @brief Generates one oscillator sample at a supplied normalized phase using BLEP correction where needed.
' */
Private Function RiffOscillatorSampleAtPhase(ByVal waveType As Long, ByVal phase As Double, ByVal dt As Double) As Single
    Dim sample As Double

    Select Case waveType
        Case 0
            sample = Sin(phase * PI2)
        Case 1
            If phase < 0.5 Then
                sample = 1#
            Else
                sample = -1#
            End If
            sample = sample + RiffPolyBLEP(phase, dt)
            Dim t2 As Double
            t2 = phase + 0.5
            If t2 >= 1# Then
                t2 = t2 - 1#
            End If
            sample = sample - RiffPolyBLEP(t2, dt)
        Case 2
            sample = (2# * phase) - 1#
            sample = sample - RiffPolyBLEP(phase, dt)
        Case Else
            sample = (Rnd() * 2!) - 1!
    End Select

    RiffOscillatorSampleAtPhase = CSng(sample)
End Function


'/**
' * @function RiffPlay
' * @brief Plays a pre-loaded static audio buffer on an available polyphonic voice channel.
' * @param bufferHandle The target buffer.
' * @return {Long} Voice handle (0-31), or -1 if all voices are occupied.
' */
Public Function RiffPlay(ByVal bufferHandle As Long) As Long
    RiffPlay = -1
    
    If Not rCtx.Initialized Then
        Exit Function
    End If
    If bufferHandle < 0 Or bufferHandle > 63 Then
        Exit Function
    End If
    If Not rCtx.Buffers(bufferHandle).Active Then
        Exit Function
    End If
    
    Dim voiceSlot As Long
    voiceSlot = InternalGetFreeVoice()
    
    If voiceSlot = -1 Then
        Exit Function
    End If
    
    InternalResetVoiceDSP voiceSlot
    
    rVoices(voiceSlot).IsOscillator = False
    rVoices(voiceSlot).BufferIndex = bufferHandle
    rVoices(voiceSlot).Position = 0#
    rVoices(voiceSlot).loopEnd = CDbl(rCtx.Buffers(bufferHandle).BufferLen)
    rVoices(voiceSlot).Playing = True
    rVoices(voiceSlot).Active = True
    
    RiffPlay = voiceSlot
End Function

'/**
' * @function RiffPlayOscillator
' * @brief Generates and plays a synthesized waveform instantly via math.
' * @param waveType 0=Sine, 1=Square, 2=Sawtooth, 3=Noise.
' * @param frequencyHz Pitch frequency in Hz.
' * @return {Long} Voice handle (0-31), or -1 if all voices are occupied.
' */
Public Function RiffPlayOscillator(ByVal waveType As Long, ByVal frequencyHz As Single) As Long
    RiffPlayOscillator = -1
    
    If Not rCtx.Initialized Then
        Exit Function
    End If
    
    Dim voiceSlot As Long
    voiceSlot = InternalGetFreeVoice()
    
    If voiceSlot = -1 Then
        Exit Function
    End If
    
    InternalResetVoiceDSP voiceSlot
    
    If waveType < 0 Then
        waveType = 0
    End If
    If waveType > 3 Then
        waveType = 3
    End If
    
    If frequencyHz < 1! Then
        frequencyHz = 440!
    End If
    
    rVoices(voiceSlot).IsOscillator = True
    rVoices(voiceSlot).OscType = waveType
    rVoices(voiceSlot).OscFreq = frequencyHz
    rVoices(voiceSlot).OscPhase = 0#
    rVoices(voiceSlot).BufferIndex = -1
    
    rVoices(voiceSlot).Playing = True
    rVoices(voiceSlot).Active = True
    
    RiffPlayOscillator = voiceSlot
End Function

'/**
' * @function InternalGetFreeVoice
' * @brief Scans for an inactive voice channel.
' * @return {Long} Voice index, or -1.
' */
Private Function InternalGetFreeVoice() As Long
    Dim i As Long
    InternalGetFreeVoice = -1
    
    For i = 0 To 31
        If Not rVoices(i).Active Then
            InternalGetFreeVoice = i
            Exit For
        End If
    Next i
End Function

'/**
' * @function InternalResetVoiceDSP
' * @brief Resets all DSP filters and engine states to neutral values for a voice.
' * @param slot The target voice index.
' */
Private Sub InternalResetVoiceDSP(ByVal slot As Long)
    rVoices(slot).Volume = 1!
    rVoices(slot).Pitch = 1#
    rVoices(slot).Pan = 0!
    rVoices(slot).Paused = False
    rVoices(slot).busID = 0
    rVoices(slot).PeakL = 0!
    rVoices(slot).PeakR = 0!

    rVoices(slot).Distortion = 1!
    rVoices(slot).lowPass = 1!
    rVoices(slot).highPass = 0!
    rVoices(slot).FilterStateL = 0!
    rVoices(slot).FilterStateR = 0!
    rVoices(slot).FilterStateHP_L = 0!
    rVoices(slot).FilterStateHP_R = 0!
    rVoices(slot).StereoWidth = 1!

    rVoices(slot).EqBass = 1!
    rVoices(slot).EqMid = 1!
    rVoices(slot).EqTreble = 1!
    rVoices(slot).EqStateLowL = 0!
    rVoices(slot).EqStateLowR = 0!
    rVoices(slot).EqStateHighL = 0!
    rVoices(slot).EqStateHighR = 0!

    rVoices(slot).CompThreshold = 1!
    rVoices(slot).CompRatio = 1!
    rVoices(slot).CompEnv = 0.0001!

    rVoices(slot).BitcrushSteps = 0!
    rVoices(slot).BitcrushDownsample = 1
    rVoices(slot).BitcrushDsCount = 0
    rVoices(slot).BitcrushLastL = 0!
    rVoices(slot).BitcrushLastR = 0!

    rVoices(slot).RingModFreq = 0!
    rVoices(slot).RingModMix = 0!
    rVoices(slot).RingModPhase = 0#

    rVoices(slot).TremoloRate = 0!
    rVoices(slot).TremoloDepth = 0!
    rVoices(slot).TremoloPhase = 0#

    rVoices(slot).AutoPanRate = 0!
    rVoices(slot).AutoPanDepth = 0!
    rVoices(slot).AutoPanPhase = 0#

    rVoices(slot).ChorusDepth = 0!
    rVoices(slot).ChorusRate = 1.5!
    rVoices(slot).ChorusPhase = 0#

    rVoices(slot).FlangerRate = 0.5!
    rVoices(slot).FlangerDepth = 0!
    rVoices(slot).FlangerFeedback = 0!
    rVoices(slot).FlangerPhase = 0#

    rVoices(slot).ReverbMix = 0!
    rVoices(slot).ReverbTime = 0.5!
    rVoices(slot).RevTap1 = Int(0.0297 * rCtx.sampleRate) * 2
    rVoices(slot).RevTap2 = Int(0.0371 * rCtx.sampleRate) * 2
    rVoices(slot).RevTap3 = Int(0.0411 * rCtx.sampleRate) * 2
    rVoices(slot).RevTap4 = Int(0.0437 * rCtx.sampleRate) * 2
    rVoices(slot).RevDamp1L = 0!
    rVoices(slot).RevDamp1R = 0!
    rVoices(slot).RevDamp2L = 0!
    rVoices(slot).RevDamp2R = 0!
    rVoices(slot).RevDamp3L = 0!
    rVoices(slot).RevDamp3R = 0!
    rVoices(slot).RevDamp4L = 0!
    rVoices(slot).RevDamp4R = 0!

    rVoices(slot).BqLowPassZ1L = 0!
    rVoices(slot).BqLowPassZ2L = 0!
    rVoices(slot).BqLowPassZ1R = 0!
    rVoices(slot).BqLowPassZ2R = 0!
    rVoices(slot).BqHighPassZ1L = 0!
    rVoices(slot).BqHighPassZ2L = 0!
    rVoices(slot).BqHighPassZ1R = 0!
    rVoices(slot).BqHighPassZ2R = 0!
    rVoices(slot).EqBassZ1L = 0!
    rVoices(slot).EqBassZ2L = 0!
    rVoices(slot).EqBassZ1R = 0!
    rVoices(slot).EqBassZ2R = 0!
    rVoices(slot).EqMidZ1L = 0!
    rVoices(slot).EqMidZ2L = 0!
    rVoices(slot).EqMidZ1R = 0!
    rVoices(slot).EqMidZ2R = 0!
    rVoices(slot).EqTrebleZ1L = 0!
    rVoices(slot).EqTrebleZ2L = 0!
    rVoices(slot).EqTrebleZ1R = 0!
    rVoices(slot).EqTrebleZ2R = 0!

    rVoices(slot).DelayTime = 0!
    rVoices(slot).DelayFeedback = 0!
    rVoices(slot).DelayMix = 0!
    rVoices(slot).RingWritePos = 0

    rVoices(slot).Looping = False
    rVoices(slot).loopStart = 0#
    rVoices(slot).fadeState = 0
    rVoices(slot).FadeFramesTotal = 0
    rVoices(slot).FadeFramesCurrent = 0

    RtlZeroMemory VarPtr(rRingBuf(slot * 192000)), 192000 * 4
End Sub

'/**
' * @function RiffPause
' * @brief Pauses a specific voice without killing it.
' * @param voiceHandle The active voice.
' */
Public Sub RiffPause(ByVal voiceHandle As Long)
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Sub
    End If
    
    If rVoices(voiceHandle).Active Then
        rVoices(voiceHandle).Paused = True
    End If
End Sub

'/**
' * @function RiffResume
' * @brief Resumes a paused voice.
' * @param voiceHandle The paused voice.
' */
Public Sub RiffResume(ByVal voiceHandle As Long)
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Sub
    End If
    
    If rVoices(voiceHandle).Active Then
        rVoices(voiceHandle).Paused = False
    End If
End Sub

'/**
' * @function RiffStop
' * @brief Immediately halts and frees a playing voice.
' * @param voiceHandle The active voice.
' */
Public Sub RiffStop(ByVal voiceHandle As Long)
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Sub
    End If
    
    rVoices(voiceHandle).Playing = False
    rVoices(voiceHandle).Active = False
End Sub

'/**
' * @function RiffStopAll
' * @brief Instantly stops all playing voices.
' */
Public Sub RiffStopAll()
    Dim i As Long
    For i = 0 To 31
        rVoices(i).Playing = False
        rVoices(i).Active = False
    Next i
End Sub

'/**
' * @function RiffFadeIn
' * @brief Smoothly fades in a voice over the specified duration.
' * @param voiceHandle The active voice.
' * @param durationSec The time in seconds to reach full volume.
' */
Public Sub RiffFadeIn(ByVal voiceHandle As Long, ByVal durationSec As Single)
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Sub
    End If
    If durationSec <= 0! Then
        Exit Sub
    End If
    
    rVoices(voiceHandle).FadeFramesTotal = CLng(durationSec * rCtx.sampleRate)
    rVoices(voiceHandle).FadeFramesCurrent = 0
    rVoices(voiceHandle).fadeState = 1
End Sub

'/**
' * @function RiffFadeOut
' * @brief Smoothly fades out a voice over the specified duration, stopping it at the end.
' * @param voiceHandle The active voice.
' * @param durationSec The time in seconds to reach zero volume.
' */
Public Sub RiffFadeOut(ByVal voiceHandle As Long, ByVal durationSec As Single)
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Sub
    End If
    If durationSec <= 0! Then
        Exit Sub
    End If
    
    rVoices(voiceHandle).FadeFramesTotal = CLng(durationSec * rCtx.sampleRate)
    rVoices(voiceHandle).FadeFramesCurrent = 0
    rVoices(voiceHandle).fadeState = 2
End Sub

'/**
' * @function RiffSetLoopRegionSec
' * @brief Constrains the playback to loop between start and end seconds.
' * @param voiceHandle The active voice.
' * @param startSec Loop start position in seconds.
' * @param endSec Loop end position in seconds.
' */
Public Sub RiffSetLoopRegionSec(ByVal voiceHandle As Long, ByVal startSec As Single, ByVal endSec As Single)
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Sub
    End If
    If rVoices(voiceHandle).IsOscillator Then
        Exit Sub
    End If
    
    Dim sByte As Double
    Dim eByte As Double
    
    sByte = startSec * rCtx.AvgBytesPerSec
    eByte = endSec * rCtx.AvgBytesPerSec
    
    If sByte < 0 Then
        sByte = 0
    End If
    
    Dim bufHandle As Long
    bufHandle = rVoices(voiceHandle).BufferIndex
    If bufHandle < 0 Or bufHandle > 63 Then
        Exit Sub
    End If
    If Not rCtx.Buffers(bufHandle).Active Then
        Exit Sub
    End If
    
    If eByte > rCtx.Buffers(bufHandle).BufferLen Then
        eByte = rCtx.Buffers(bufHandle).BufferLen
    End If
    
    Dim nBlockAlign As Integer
    RtlMoveMemoryToInteger nBlockAlign, ByVal (rCtx.MixFormatPtr + 12), 2
    
    Dim align As Long
    align = CLng(nBlockAlign)
    
    sByte = CDbl((CLng(sByte) \ align) * align)
    eByte = CDbl((CLng(eByte) \ align) * align)
    
    rVoices(voiceHandle).loopStart = sByte
    rVoices(voiceHandle).loopEnd = eByte
End Sub


'/**
' * @property RiffVoiceIsPlaying
' * @brief Checks if a voice is actively processing audio.
' */
Public Property Get RiffVoiceIsPlaying(ByVal voiceHandle As Long) As Boolean
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    RiffVoiceIsPlaying = (rVoices(voiceHandle).Active And rVoices(voiceHandle).Playing)
End Property

'/**
' * @property RiffVoiceIsPaused
' * @brief Checks if a voice is currently paused.
' */
Public Property Get RiffVoiceIsPaused(ByVal voiceHandle As Long) As Boolean
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    RiffVoiceIsPaused = rVoices(voiceHandle).Paused
End Property

'/**
' * @property RiffVoiceBus
' * @brief Determines which Audio Bus this voice routes to (0 to 7).
' */
Public Property Get RiffVoiceBus(ByVal voiceHandle As Long) As Long
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    RiffVoiceBus = rVoices(voiceHandle).busID
End Property
Public Property Let RiffVoiceBus(ByVal voiceHandle As Long, ByVal value As Long)
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    
    If value < 0 Then
        value = 0
    End If
    If value > 7 Then
        value = 7
    End If
    
    rVoices(voiceHandle).busID = value
End Property

'/**
' * @function RiffVoiceGetPeak
' * @brief Retrieves the instantaneous Peak Amplitude for this voice's VU Meters.
' * @param peakLeft Variable to store the left channel peak.
' * @param peakRight Variable to store the right channel peak.
' */
Public Sub RiffVoiceGetPeak(ByVal voiceHandle As Long, ByRef peakLeft As Single, ByRef peakRight As Single)
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        peakLeft = 0!
        peakRight = 0!
        Exit Sub
    End If
    
    peakLeft = rVoices(voiceHandle).PeakL
    peakRight = rVoices(voiceHandle).PeakR
End Sub

'/**
' * @property RiffVoiceLoop
' * @brief Defines whether the voice should restart automatically when it reaches the end.
' */
Public Property Get RiffVoiceLoop(ByVal voiceHandle As Long) As Boolean
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    RiffVoiceLoop = rVoices(voiceHandle).Looping
End Property
Public Property Let RiffVoiceLoop(ByVal voiceHandle As Long, ByVal value As Boolean)
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    rVoices(voiceHandle).Looping = value
End Property

'/**
' * @property RiffVoicePositionSec
' * @brief Current playback position in seconds.
' */
Public Property Get RiffVoicePositionSec(ByVal voiceHandle As Long) As Single
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Or rCtx.AvgBytesPerSec = 0 Then
        Exit Property
    End If
    RiffVoicePositionSec = CSng(rVoices(voiceHandle).Position) / CSng(rCtx.AvgBytesPerSec)
End Property
Public Property Let RiffVoicePositionSec(ByVal voiceHandle As Long, ByVal value As Single)
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    If rVoices(voiceHandle).IsOscillator Then
        Exit Property
    End If
    
    Dim posBytes As Double
    posBytes = CDbl(value) * rCtx.AvgBytesPerSec
    
    If posBytes < 0 Then
        posBytes = 0
    End If
    
    Dim bufHandle As Long
    bufHandle = rVoices(voiceHandle).BufferIndex
    If bufHandle < 0 Or bufHandle > 63 Then
        Exit Property
    End If
    If Not rCtx.Buffers(bufHandle).Active Then
        Exit Property
    End If
    
    If posBytes >= rCtx.Buffers(bufHandle).BufferLen Then
        posBytes = rCtx.Buffers(bufHandle).BufferLen - 1
    End If
    
    rVoices(voiceHandle).Position = posBytes
End Property

'/**
' * @property RiffVoiceVolume
' * @brief Individual volume for this voice (0.0 to 1.0).
' */
Public Property Get RiffVoiceVolume(ByVal voiceHandle As Long) As Single
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    RiffVoiceVolume = rVoices(voiceHandle).Volume
End Property
Public Property Let RiffVoiceVolume(ByVal voiceHandle As Long, ByVal value As Single)
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    
    If value < 0! Then
        value = 0!
    End If
    If value > 1! Then
        value = 1!
    End If
    
    rVoices(voiceHandle).Volume = value
End Property

'/**
' * @property RiffVoicePitch
' * @brief Speed/Pitch modifier (1.0 = Normal, 2.0 = Double speed/octave higher).
' */
Public Property Get RiffVoicePitch(ByVal voiceHandle As Long) As Single
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    RiffVoicePitch = CSng(rVoices(voiceHandle).Pitch)
End Property
Public Property Let RiffVoicePitch(ByVal voiceHandle As Long, ByVal value As Single)
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    
    If value <= 0.1! Then
        value = 0.1!
    End If
    
    rVoices(voiceHandle).Pitch = CDbl(value)
End Property

'/**
' * @property RiffVoicePan
' * @brief Stereo panning (-1.0 = Left, 0.0 = Center, 1.0 = Right).
' */
Public Property Get RiffVoicePan(ByVal voiceHandle As Long) As Single
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    RiffVoicePan = rVoices(voiceHandle).Pan
End Property
Public Property Let RiffVoicePan(ByVal voiceHandle As Long, ByVal value As Single)
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    
    If value < -1! Then
        value = -1!
    End If
    If value > 1! Then
        value = 1!
    End If
    
    rVoices(voiceHandle).Pan = value
End Property


'/**
' * @property RiffVoiceBitDepth
' * @brief Simulates retro console audio by quantizing amplitude bits.
' */
Public Property Get RiffVoiceBitDepth(ByVal voiceHandle As Long) As Single
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    
    If rVoices(voiceHandle).BitcrushSteps = 0! Then
        RiffVoiceBitDepth = 32!
    Else
        RiffVoiceBitDepth = Log(rVoices(voiceHandle).BitcrushSteps) / Log(2)
    End If
End Property
Public Property Let RiffVoiceBitDepth(ByVal voiceHandle As Long, ByVal value As Single)
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    
    If value >= 32! Then
        rVoices(voiceHandle).BitcrushSteps = 0!
        Exit Property
    End If
    If value < 2! Then
        value = 2!
    End If
    
    rVoices(voiceHandle).BitcrushSteps = 2 ^ value
End Property

'/**
' * @property RiffVoiceSampleRateReduction
' * @brief Creates robotic artifacts by holding samples for N frames.
' */
Public Property Get RiffVoiceSampleRateReduction(ByVal voiceHandle As Long) As Long
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    RiffVoiceSampleRateReduction = rVoices(voiceHandle).BitcrushDownsample
End Property
Public Property Let RiffVoiceSampleRateReduction(ByVal voiceHandle As Long, ByVal value As Long)
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    If value < 1 Then
        value = 1
    End If
    rVoices(voiceHandle).BitcrushDownsample = value
End Property

'/**
' * @property RiffVoiceRingModFreq
' * @brief The frequency in Hz for the Ring Modulator oscillator.
' */
Public Property Get RiffVoiceRingModFreq(ByVal voiceHandle As Long) As Single
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    RiffVoiceRingModFreq = rVoices(voiceHandle).RingModFreq
End Property
Public Property Let RiffVoiceRingModFreq(ByVal voiceHandle As Long, ByVal value As Single)
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    If value < 0! Then
        value = 0!
    End If
    rVoices(voiceHandle).RingModFreq = value
End Property

'/**
' * @property RiffVoiceRingModMix
' * @brief The blend amount of the Ring Modulator effect (0.0 to 1.0).
' */
Public Property Get RiffVoiceRingModMix(ByVal voiceHandle As Long) As Single
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    RiffVoiceRingModMix = rVoices(voiceHandle).RingModMix
End Property
Public Property Let RiffVoiceRingModMix(ByVal voiceHandle As Long, ByVal value As Single)
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    If value < 0! Then
        value = 0!
    End If
    If value > 1! Then
        value = 1!
    End If
    rVoices(voiceHandle).RingModMix = value
End Property

'/**
' * @property RiffVoiceAutoPanRate
' * @brief Speed of the automatic panning LFO in Hz.
' */
Public Property Get RiffVoiceAutoPanRate(ByVal voiceHandle As Long) As Single
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    RiffVoiceAutoPanRate = rVoices(voiceHandle).AutoPanRate
End Property
Public Property Let RiffVoiceAutoPanRate(ByVal voiceHandle As Long, ByVal value As Single)
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    If value < 0! Then
        value = 0!
    End If
    If value > 20! Then
        value = 20!
    End If
    rVoices(voiceHandle).AutoPanRate = value
End Property

'/**
' * @property RiffVoiceAutoPanDepth
' * @brief Intensity of the auto-panning effect.
' */
Public Property Get RiffVoiceAutoPanDepth(ByVal voiceHandle As Long) As Single
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    RiffVoiceAutoPanDepth = rVoices(voiceHandle).AutoPanDepth
End Property
Public Property Let RiffVoiceAutoPanDepth(ByVal voiceHandle As Long, ByVal value As Single)
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    If value < 0! Then
        value = 0!
    End If
    If value > 1! Then
        value = 1!
    End If
    rVoices(voiceHandle).AutoPanDepth = value
End Property

'/**
' * @property RiffVoiceEqBass
' * @brief Low frequency shelf gain (1.0 is flat).
' */
Public Property Get RiffVoiceEqBass(ByVal voiceHandle As Long) As Single
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    RiffVoiceEqBass = rVoices(voiceHandle).EqBass
End Property
Public Property Let RiffVoiceEqBass(ByVal voiceHandle As Long, ByVal value As Single)
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    If value < 0! Then
        value = 0!
    End If
    If value > 5! Then
        value = 5!
    End If
    rVoices(voiceHandle).EqBass = value
End Property

'/**
' * @property RiffVoiceEqMid
' * @brief Mid frequency band gain (1.0 is flat).
' */
Public Property Get RiffVoiceEqMid(ByVal voiceHandle As Long) As Single
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    RiffVoiceEqMid = rVoices(voiceHandle).EqMid
End Property
Public Property Let RiffVoiceEqMid(ByVal voiceHandle As Long, ByVal value As Single)
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    If value < 0! Then
        value = 0!
    End If
    If value > 5! Then
        value = 5!
    End If
    rVoices(voiceHandle).EqMid = value
End Property

'/**
' * @property RiffVoiceEqTreble
' * @brief High frequency shelf gain (1.0 is flat).
' */
Public Property Get RiffVoiceEqTreble(ByVal voiceHandle As Long) As Single
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    RiffVoiceEqTreble = rVoices(voiceHandle).EqTreble
End Property
Public Property Let RiffVoiceEqTreble(ByVal voiceHandle As Long, ByVal value As Single)
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    If value < 0! Then
        value = 0!
    End If
    If value > 5! Then
        value = 5!
    End If
    rVoices(voiceHandle).EqTreble = value
End Property

'/**
' * @property RiffVoiceCompressorThreshold
' * @brief Volume level at which the compressor starts reducing gain.
' */
Public Property Get RiffVoiceCompressorThreshold(ByVal voiceHandle As Long) As Single
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    RiffVoiceCompressorThreshold = rVoices(voiceHandle).CompThreshold
End Property
Public Property Let RiffVoiceCompressorThreshold(ByVal voiceHandle As Long, ByVal value As Single)
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    If value <= 0! Then
        value = 0.01!
    End If
    If value > 1! Then
        value = 1!
    End If
    rVoices(voiceHandle).CompThreshold = value
End Property

'/**
' * @property RiffVoiceCompressorRatio
' * @brief Amount of gain reduction applied when signal exceeds the threshold.
' */
Public Property Get RiffVoiceCompressorRatio(ByVal voiceHandle As Long) As Single
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    RiffVoiceCompressorRatio = rVoices(voiceHandle).CompRatio
End Property
Public Property Let RiffVoiceCompressorRatio(ByVal voiceHandle As Long, ByVal value As Single)
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    If value < 1! Then
        value = 1!
    End If
    If value > 20! Then
        value = 20!
    End If
    rVoices(voiceHandle).CompRatio = value
End Property

'/**
' * @property RiffVoiceFlangerDepth
' * @brief Blend amount for the Flanger effect.
' */
Public Property Get RiffVoiceFlangerDepth(ByVal voiceHandle As Long) As Single
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    RiffVoiceFlangerDepth = rVoices(voiceHandle).FlangerDepth
End Property
Public Property Let RiffVoiceFlangerDepth(ByVal voiceHandle As Long, ByVal value As Single)
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    If value < 0! Then
        value = 0!
    End If
    If value > 1! Then
        value = 1!
    End If
    rVoices(voiceHandle).FlangerDepth = value
End Property

'/**
' * @property RiffVoiceFlangerRate
' * @brief Sweep rate of the Flanger in Hz.
' */
Public Property Get RiffVoiceFlangerRate(ByVal voiceHandle As Long) As Single
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    RiffVoiceFlangerRate = rVoices(voiceHandle).FlangerRate
End Property
Public Property Let RiffVoiceFlangerRate(ByVal voiceHandle As Long, ByVal value As Single)
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    If value < 0.1! Then
        value = 0.1!
    End If
    If value > 10! Then
        value = 10!
    End If
    rVoices(voiceHandle).FlangerRate = value
End Property

'/**
' * @property RiffVoiceFlangerFeedback
' * @brief Resonance intensity of the Flanger effect.
' */
Public Property Get RiffVoiceFlangerFeedback(ByVal voiceHandle As Long) As Single
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    RiffVoiceFlangerFeedback = rVoices(voiceHandle).FlangerFeedback
End Property
Public Property Let RiffVoiceFlangerFeedback(ByVal voiceHandle As Long, ByVal value As Single)
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    If value < 0! Then
        value = 0!
    End If
    If value > 0.95! Then
        value = 0.95!
    End If
    rVoices(voiceHandle).FlangerFeedback = value
End Property

'/**
' * @property RiffVoiceDistortion
' * @brief Digital clipping multiplier.
' */
Public Property Get RiffVoiceDistortion(ByVal voiceHandle As Long) As Single
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    RiffVoiceDistortion = rVoices(voiceHandle).Distortion
End Property
Public Property Let RiffVoiceDistortion(ByVal voiceHandle As Long, ByVal value As Single)
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    If value < 1! Then
        value = 1!
    End If
    rVoices(voiceHandle).Distortion = value
End Property

'/**
' * @property RiffVoiceLowPass
' * @brief Muffles the audio by filtering high frequencies.
' */
Public Property Get RiffVoiceLowPass(ByVal voiceHandle As Long) As Single
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    RiffVoiceLowPass = rVoices(voiceHandle).lowPass
End Property
Public Property Let RiffVoiceLowPass(ByVal voiceHandle As Long, ByVal value As Single)
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    If value <= 0! Then
        value = 0.01!
    End If
    If value > 1! Then
        value = 1!
    End If
    rVoices(voiceHandle).lowPass = value
End Property

'/**
' * @property RiffVoiceHighPass
' * @brief Thins out the audio by filtering low frequencies.
' */
Public Property Get RiffVoiceHighPass(ByVal voiceHandle As Long) As Single
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    RiffVoiceHighPass = rVoices(voiceHandle).highPass
End Property
Public Property Let RiffVoiceHighPass(ByVal voiceHandle As Long, ByVal value As Single)
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    If value < 0! Then
        value = 0!
    End If
    If value > 0.99! Then
        value = 0.99!
    End If
    rVoices(voiceHandle).highPass = value
End Property

'/**
' * @property RiffVoiceStereoWidth
' * @brief Adjusts the perceived width of the stereo field.
' */
Public Property Get RiffVoiceStereoWidth(ByVal voiceHandle As Long) As Single
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    RiffVoiceStereoWidth = rVoices(voiceHandle).StereoWidth
End Property
Public Property Let RiffVoiceStereoWidth(ByVal voiceHandle As Long, ByVal value As Single)
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    If value < 0! Then
        value = 0!
    End If
    If value > 5! Then
        value = 5!
    End If
    rVoices(voiceHandle).StereoWidth = value
End Property

'/**
' * @property RiffVoiceTremoloRate
' * @brief Speed of the volume oscillation LFO in Hz.
' */
Public Property Get RiffVoiceTremoloRate(ByVal voiceHandle As Long) As Single
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    RiffVoiceTremoloRate = rVoices(voiceHandle).TremoloRate
End Property
Public Property Let RiffVoiceTremoloRate(ByVal voiceHandle As Long, ByVal value As Single)
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    If value < 0! Then
        value = 0!
    End If
    If value > 20! Then
        value = 20!
    End If
    rVoices(voiceHandle).TremoloRate = value
End Property

'/**
' * @property RiffVoiceTremoloDepth
' * @brief Intensity of the volume oscillation effect.
' */
Public Property Get RiffVoiceTremoloDepth(ByVal voiceHandle As Long) As Single
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    RiffVoiceTremoloDepth = rVoices(voiceHandle).TremoloDepth
End Property
Public Property Let RiffVoiceTremoloDepth(ByVal voiceHandle As Long, ByVal value As Single)
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    If value < 0! Then
        value = 0!
    End If
    If value > 1! Then
        value = 1!
    End If
    rVoices(voiceHandle).TremoloDepth = value
End Property

'/**
' * @property RiffVoiceChorusDepth
' * @brief Wet mix amount for the multi-voice Chorus effect.
' */
Public Property Get RiffVoiceChorusDepth(ByVal voiceHandle As Long) As Single
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    RiffVoiceChorusDepth = rVoices(voiceHandle).ChorusDepth
End Property
Public Property Let RiffVoiceChorusDepth(ByVal voiceHandle As Long, ByVal value As Single)
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    If value < 0! Then
        value = 0!
    End If
    If value > 1! Then
        value = 1!
    End If
    rVoices(voiceHandle).ChorusDepth = value
End Property

'/**
' * @property RiffVoiceChorusRate
' * @brief LFO rate governing the Chorus pitch modulation.
' */
Public Property Get RiffVoiceChorusRate(ByVal voiceHandle As Long) As Single
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    RiffVoiceChorusRate = rVoices(voiceHandle).ChorusRate
End Property
Public Property Let RiffVoiceChorusRate(ByVal voiceHandle As Long, ByVal value As Single)
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    If value < 0.1! Then
        value = 0.1!
    End If
    If value > 10! Then
        value = 10!
    End If
    rVoices(voiceHandle).ChorusRate = value
End Property

'/**
' * @property RiffVoiceReverbMix
' * @brief Blend of the simulated spatial room reverberation.
' */
Public Property Get RiffVoiceReverbMix(ByVal voiceHandle As Long) As Single
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    RiffVoiceReverbMix = rVoices(voiceHandle).ReverbMix
End Property
Public Property Let RiffVoiceReverbMix(ByVal voiceHandle As Long, ByVal value As Single)
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    If value < 0! Then
        value = 0!
    End If
    If value > 1! Then
        value = 1!
    End If
    rVoices(voiceHandle).ReverbMix = value
End Property

'/**
' * @property RiffVoiceReverbTime
' * @brief Determines the decay length / simulated room size.
' */
Public Property Get RiffVoiceReverbTime(ByVal voiceHandle As Long) As Single
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    RiffVoiceReverbTime = rVoices(voiceHandle).ReverbTime
End Property
Public Property Let RiffVoiceReverbTime(ByVal voiceHandle As Long, ByVal value As Single)
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    If value < 0! Then
        value = 0!
    End If
    If value > 0.95! Then
        value = 0.95!
    End If
    rVoices(voiceHandle).ReverbTime = value
End Property

'/**
' * @property RiffVoiceDelayTime
' * @brief Interval between consecutive delay echoes.
' */
Public Property Get RiffVoiceDelayTime(ByVal voiceHandle As Long) As Single
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    RiffVoiceDelayTime = rVoices(voiceHandle).DelayTime
End Property
Public Property Let RiffVoiceDelayTime(ByVal voiceHandle As Long, ByVal value As Single)
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    If value < 0! Then
        value = 0!
    End If
    If value > 1! Then
        value = 1!
    End If
    rVoices(voiceHandle).DelayTime = value
End Property

'/**
' * @property RiffVoiceDelayFeedback
' * @brief The amount of signal fed back into the delay line to create decaying echoes.
' */
Public Property Get RiffVoiceDelayFeedback(ByVal voiceHandle As Long) As Single
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    RiffVoiceDelayFeedback = rVoices(voiceHandle).DelayFeedback
End Property
Public Property Let RiffVoiceDelayFeedback(ByVal voiceHandle As Long, ByVal value As Single)
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    If value < 0! Then
        value = 0!
    End If
    If value > 0.95! Then
        value = 0.95!
    End If
    rVoices(voiceHandle).DelayFeedback = value
End Property

'/**
' * @property RiffVoiceDelayMix
' * @brief Blend of the Echo/Delay effect into the main output.
' */
Public Property Get RiffVoiceDelayMix(ByVal voiceHandle As Long) As Single
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    RiffVoiceDelayMix = rVoices(voiceHandle).DelayMix
End Property
Public Property Let RiffVoiceDelayMix(ByVal voiceHandle As Long, ByVal value As Single)
    If Not rCtx.Initialized Or voiceHandle < 0 Or voiceHandle > 31 Then
        Exit Property
    End If
    If value < 0! Then
        value = 0!
    End If
    If value > 1! Then
        value = 1!
    End If
    rVoices(voiceHandle).DelayMix = value
End Property


'/**
' * @function RiffTimerCallback
' * @brief Core multimedia timer callback. Executes the DSP pipeline and writes to WASAPI.
' * @param hWnd Window Handle (not used).
' * @param uMsg System Message (not used).
' * @param idEvent Timer Identifier (not used).
' * @param dwTime System Time (not used).
' */

'/**
' * @function RiffClamp
' * @brief Bounds a floating-point value into a deterministic range for DSP safety.
' */
Private Function RiffClamp(ByVal value As Single, ByVal minValue As Single, ByVal maxValue As Single) As Single
    If value < minValue Then
        RiffClamp = minValue
    ElseIf value > maxValue Then
        RiffClamp = maxValue
    Else
        RiffClamp = value
    End If
End Function

'/**
' * @function RiffPolyBLEP
' * @brief Produces a polynomial band-limiting correction at waveform discontinuities.
' */
Private Function RiffPolyBLEP(ByVal t As Double, ByVal dt As Double) As Single
    If dt <= 0# Then
        Exit Function
    End If

    If t < dt Then
        t = t / dt
        RiffPolyBLEP = CSng((t + t) - (t * t) - 1#)
    ElseIf t > 1# - dt Then
        t = (t - 1#) / dt
        RiffPolyBLEP = CSng((t * t) + (t + t) + 1#)
    Else
        RiffPolyBLEP = 0!
    End If
End Function

'/**
' * @function RiffNextOscillatorSample
' * @brief Generates one oscillator sample using BLEP band-limiting for discontinuous waveforms.
' */
Private Function RiffNextOscillatorSample(ByVal voiceIndex As Long) As Single
    Dim dt As Double
    Dim phase01 As Double
    Dim sample As Single
    Dim edge As Double

    If rCtx.sampleRate <= 0 Then
        Exit Function
    End If

    dt = CDbl(rVoices(voiceIndex).OscFreq) / CDbl(rCtx.sampleRate)
    If dt <= 0# Then
        dt = 440# / CDbl(rCtx.sampleRate)
    End If
    If dt > 0.5 Then
        dt = 0.5
    End If

    phase01 = rVoices(voiceIndex).OscPhase / PI2
    phase01 = phase01 - Fix(phase01)
    If phase01 < 0# Then
        phase01 = phase01 + 1#
    End If

    Select Case rVoices(voiceIndex).OscType
        Case 0
            sample = CSng(Sin(rVoices(voiceIndex).OscPhase))
        Case 1
            If phase01 < 0.5 Then
                sample = 0.65!
            Else
                sample = -0.65!
            End If
            sample = sample + 0.65! * RiffPolyBLEP(phase01, dt)
            edge = phase01 + 0.5
            If edge >= 1# Then
                edge = edge - 1#
            End If
            sample = sample - 0.65! * RiffPolyBLEP(edge, dt)
        Case 2
            sample = CSng((2# * phase01) - 1#)
            sample = sample - RiffPolyBLEP(phase01, dt)
            sample = sample * 0.65!
        Case Else
            sample = (Rnd() * 2!) - 1!
    End Select

    rVoices(voiceIndex).OscPhase = rVoices(voiceIndex).OscPhase + (dt * PI2)
    If rVoices(voiceIndex).OscPhase >= PI2 Then
        rVoices(voiceIndex).OscPhase = rVoices(voiceIndex).OscPhase - PI2
    End If

    RiffNextOscillatorSample = sample
End Function

'/**
' * @function RiffBiquadProcess
' * @brief Processes one sample through a transposed direct-form II biquad section.
' */
Private Function RiffBiquadProcess(ByVal inputSample As Single, ByRef z1 As Single, ByRef z2 As Single, ByVal b0 As Single, ByVal b1 As Single, ByVal b2 As Single, ByVal a1 As Single, ByVal a2 As Single) As Single
    Dim y As Single

    y = (b0 * inputSample) + z1
    z1 = (b1 * inputSample) - (a1 * y) + z2
    z2 = (b2 * inputSample) - (a2 * y)
    RiffBiquadProcess = y
End Function

'/**
' * @function RiffBiquadLowPassCoeffs
' * @brief Calculates normalized second-order low-pass filter coefficients.
' */
Private Sub RiffBiquadLowPassCoeffs(ByVal cutoffHz As Single, ByVal q As Single, ByRef b0 As Single, ByRef b1 As Single, ByRef b2 As Single, ByRef a1 As Single, ByRef a2 As Single)
    Dim omega As Double
    Dim sn As Double
    Dim cs As Double
    Dim alpha As Double
    Dim a0 As Double

    cutoffHz = RiffClamp(cutoffHz, 20!, CSng(rCtx.sampleRate) * 0.45!)
    q = RiffClamp(q, 0.1!, 12!)

    omega = PI2 * CDbl(cutoffHz) / CDbl(rCtx.sampleRate)
    sn = Sin(omega)
    cs = Cos(omega)
    alpha = sn / (2# * CDbl(q))
    a0 = 1# + alpha

    b0 = CSng(((1# - cs) * 0.5) / a0)
    b1 = CSng((1# - cs) / a0)
    b2 = b0
    a1 = CSng((-2# * cs) / a0)
    a2 = CSng((1# - alpha) / a0)
End Sub

'/**
' * @function RiffBiquadHighPassCoeffs
' * @brief Calculates normalized second-order high-pass filter coefficients.
' */
Private Sub RiffBiquadHighPassCoeffs(ByVal cutoffHz As Single, ByVal q As Single, ByRef b0 As Single, ByRef b1 As Single, ByRef b2 As Single, ByRef a1 As Single, ByRef a2 As Single)
    Dim omega As Double
    Dim sn As Double
    Dim cs As Double
    Dim alpha As Double
    Dim a0 As Double

    cutoffHz = RiffClamp(cutoffHz, 20!, CSng(rCtx.sampleRate) * 0.45!)
    q = RiffClamp(q, 0.1!, 12!)

    omega = PI2 * CDbl(cutoffHz) / CDbl(rCtx.sampleRate)
    sn = Sin(omega)
    cs = Cos(omega)
    alpha = sn / (2# * CDbl(q))
    a0 = 1# + alpha

    b0 = CSng(((1# + cs) * 0.5) / a0)
    b1 = CSng((-(1# + cs)) / a0)
    b2 = b0
    a1 = CSng((-2# * cs) / a0)
    a2 = CSng((1# - alpha) / a0)
End Sub

'/**
' * @function RiffBiquadPeakCoeffs
' * @brief Calculates normalized parametric EQ coefficients for a single band.
' */
Private Sub RiffBiquadPeakCoeffs(ByVal freqHz As Single, ByVal q As Single, ByVal gain As Single, ByRef b0 As Single, ByRef b1 As Single, ByRef b2 As Single, ByRef a1 As Single, ByRef a2 As Single)
    Dim omega As Double
    Dim sn As Double
    Dim cs As Double
    Dim alpha As Double
    Dim amp As Double
    Dim a0 As Double

    freqHz = RiffClamp(freqHz, 20!, CSng(rCtx.sampleRate) * 0.45!)
    q = RiffClamp(q, 0.1!, 12!)
    gain = RiffClamp(gain, 0.05!, 8!)

    omega = PI2 * CDbl(freqHz) / CDbl(rCtx.sampleRate)
    sn = Sin(omega)
    cs = Cos(omega)
    alpha = sn / (2# * CDbl(q))
    amp = Sqr(CDbl(gain))
    a0 = 1# + (alpha / amp)

    b0 = CSng((1# + (alpha * amp)) / a0)
    b1 = CSng((-2# * cs) / a0)
    b2 = CSng((1# - (alpha * amp)) / a0)
    a1 = CSng((-2# * cs) / a0)
    a2 = CSng((1# - (alpha / amp)) / a0)
End Sub

'/**
' * @function RiffProcessVoiceFilters
' * @brief Applies biquad low-pass, high-pass, and three-band parametric EQ to one stereo sample.
' */
Private Sub RiffProcessVoiceFilters(ByVal voiceIndex As Long, ByRef leftSample As Single, ByRef rightSample As Single, ByVal lowPass As Single, ByVal highPass As Single, ByVal bassGain As Single, ByVal midGain As Single, ByVal trebleGain As Single)
    Dim b0 As Single
    Dim b1 As Single
    Dim b2 As Single
    Dim a1 As Single
    Dim a2 As Single
    Dim cutoff As Single

    If rCtx.sampleRate <= 0 Then
        Exit Sub
    End If

    If lowPass < 0.999! Then
        cutoff = 40! + ((RiffClamp(lowPass, 0!, 1!) ^ 2!) * ((CSng(rCtx.sampleRate) * 0.45!) - 40!))
        RiffBiquadLowPassCoeffs cutoff, 0.707!, b0, b1, b2, a1, a2
        leftSample = RiffBiquadProcess(leftSample, rVoices(voiceIndex).BqLowPassZ1L, rVoices(voiceIndex).BqLowPassZ2L, b0, b1, b2, a1, a2)
        rightSample = RiffBiquadProcess(rightSample, rVoices(voiceIndex).BqLowPassZ1R, rVoices(voiceIndex).BqLowPassZ2R, b0, b1, b2, a1, a2)
    End If

    If highPass > 0! Then
        cutoff = 20! + ((RiffClamp(highPass, 0!, 1!) ^ 2!) * ((CSng(rCtx.sampleRate) * 0.35!) - 20!))
        RiffBiquadHighPassCoeffs cutoff, 0.707!, b0, b1, b2, a1, a2
        leftSample = RiffBiquadProcess(leftSample, rVoices(voiceIndex).BqHighPassZ1L, rVoices(voiceIndex).BqHighPassZ2L, b0, b1, b2, a1, a2)
        rightSample = RiffBiquadProcess(rightSample, rVoices(voiceIndex).BqHighPassZ1R, rVoices(voiceIndex).BqHighPassZ2R, b0, b1, b2, a1, a2)
    End If

    If bassGain <> 1! Then
        RiffBiquadPeakCoeffs 120!, 0.7!, bassGain, b0, b1, b2, a1, a2
        leftSample = RiffBiquadProcess(leftSample, rVoices(voiceIndex).EqBassZ1L, rVoices(voiceIndex).EqBassZ2L, b0, b1, b2, a1, a2)
        rightSample = RiffBiquadProcess(rightSample, rVoices(voiceIndex).EqBassZ1R, rVoices(voiceIndex).EqBassZ2R, b0, b1, b2, a1, a2)
    End If

    If midGain <> 1! Then
        RiffBiquadPeakCoeffs 1000!, 1!, midGain, b0, b1, b2, a1, a2
        leftSample = RiffBiquadProcess(leftSample, rVoices(voiceIndex).EqMidZ1L, rVoices(voiceIndex).EqMidZ2L, b0, b1, b2, a1, a2)
        rightSample = RiffBiquadProcess(rightSample, rVoices(voiceIndex).EqMidZ1R, rVoices(voiceIndex).EqMidZ2R, b0, b1, b2, a1, a2)
    End If

    If trebleGain <> 1! Then
        RiffBiquadPeakCoeffs 6500!, 0.7!, trebleGain, b0, b1, b2, a1, a2
        leftSample = RiffBiquadProcess(leftSample, rVoices(voiceIndex).EqTrebleZ1L, rVoices(voiceIndex).EqTrebleZ2L, b0, b1, b2, a1, a2)
        rightSample = RiffBiquadProcess(rightSample, rVoices(voiceIndex).EqTrebleZ1R, rVoices(voiceIndex).EqTrebleZ2R, b0, b1, b2, a1, a2)
    End If
End Sub

'/**
' * @function RiffRingRead
' * @brief Reads one channel from the per-voice stereo delay line with wraparound.
' */
Private Function RiffRingRead(ByVal baseIndex As Long, ByVal readIndex As Long, ByVal channelOffset As Long) As Single
    Do While readIndex < 0
        readIndex = readIndex + 192000
    Loop
    If readIndex >= 192000 Then
        readIndex = readIndex Mod 192000
    End If
    RiffRingRead = rRingBuf(baseIndex + readIndex + channelOffset)
End Function

'/**
' * @function RiffProcessFreeverb
' * @brief Applies a Freeverb-style damped comb network with stereo cross-feed.
' */
Private Sub RiffProcessFreeverb(ByVal voiceIndex As Long, ByVal baseIndex As Long, ByVal writeIndex As Long, ByVal mix As Single, ByVal decay As Single, ByRef leftSample As Single, ByRef rightSample As Single, ByRef feedbackLeft As Single, ByRef feedbackRight As Single)
    Dim fb As Single
    Dim damp As Single
    Dim l1 As Single
    Dim r1 As Single
    Dim l2 As Single
    Dim r2 As Single
    Dim l3 As Single
    Dim r3 As Single
    Dim l4 As Single
    Dim r4 As Single
    Dim wetL As Single
    Dim wetR As Single

    fb = 0.68! + (RiffClamp(decay, 0!, 1!) * 0.28!)
    damp = 0.18! + ((1! - RiffClamp(decay, 0!, 1!)) * 0.24!)
    mix = RiffClamp(mix, 0!, 1!)

    l1 = RiffRingRead(baseIndex, writeIndex - rVoices(voiceIndex).RevTap1, 0)
    r1 = RiffRingRead(baseIndex, writeIndex - rVoices(voiceIndex).RevTap1, 1)
    l2 = RiffRingRead(baseIndex, writeIndex - rVoices(voiceIndex).RevTap2, 0)
    r2 = RiffRingRead(baseIndex, writeIndex - rVoices(voiceIndex).RevTap2, 1)
    l3 = RiffRingRead(baseIndex, writeIndex - rVoices(voiceIndex).RevTap3, 0)
    r3 = RiffRingRead(baseIndex, writeIndex - rVoices(voiceIndex).RevTap3, 1)
    l4 = RiffRingRead(baseIndex, writeIndex - rVoices(voiceIndex).RevTap4, 0)
    r4 = RiffRingRead(baseIndex, writeIndex - rVoices(voiceIndex).RevTap4, 1)

    rVoices(voiceIndex).RevDamp1L = (l1 * (1! - damp)) + (rVoices(voiceIndex).RevDamp1L * damp)
    rVoices(voiceIndex).RevDamp1R = (r1 * (1! - damp)) + (rVoices(voiceIndex).RevDamp1R * damp)
    rVoices(voiceIndex).RevDamp2L = (l2 * (1! - damp)) + (rVoices(voiceIndex).RevDamp2L * damp)
    rVoices(voiceIndex).RevDamp2R = (r2 * (1! - damp)) + (rVoices(voiceIndex).RevDamp2R * damp)
    rVoices(voiceIndex).RevDamp3L = (l3 * (1! - damp)) + (rVoices(voiceIndex).RevDamp3L * damp)
    rVoices(voiceIndex).RevDamp3R = (r3 * (1! - damp)) + (rVoices(voiceIndex).RevDamp3R * damp)
    rVoices(voiceIndex).RevDamp4L = (l4 * (1! - damp)) + (rVoices(voiceIndex).RevDamp4L * damp)
    rVoices(voiceIndex).RevDamp4R = (r4 * (1! - damp)) + (rVoices(voiceIndex).RevDamp4R * damp)

    wetL = ((rVoices(voiceIndex).RevDamp1L + rVoices(voiceIndex).RevDamp2L + rVoices(voiceIndex).RevDamp3L + rVoices(voiceIndex).RevDamp4L) * 0.19!) + ((rVoices(voiceIndex).RevDamp2R + rVoices(voiceIndex).RevDamp4R) * 0.055!)
    wetR = ((rVoices(voiceIndex).RevDamp1R + rVoices(voiceIndex).RevDamp2R + rVoices(voiceIndex).RevDamp3R + rVoices(voiceIndex).RevDamp4R) * 0.19!) + ((rVoices(voiceIndex).RevDamp1L + rVoices(voiceIndex).RevDamp3L) * 0.055!)

    leftSample = leftSample + (wetL * mix)
    rightSample = rightSample + (wetR * mix)
    feedbackLeft = feedbackLeft + (wetL * fb)
    feedbackRight = feedbackRight + (wetR * fb)
End Sub

#If VBA7 Then
Private Sub RiffTimerCallback(ByVal hWnd As LongPtr, ByVal uMsg As Long, ByVal idEvent As LongPtr, ByVal dwTime As Long)
#Else
Private Sub RiffTimerCallback(ByVal hWnd As Long, ByVal uMsg As Long, ByVal idEvent As Long, ByVal dwTime As Long)
#End If
    If rCtx.MagicCookie <> &H52494646 Then
        Exit Sub
    End If
    
    Dim padding As Long
    Dim framesAvailable As Long
    Dim hr As Long
    Dim bytesToWrite As Long
    Dim i As Long
    Dim frame As Long
    Dim readPos As Long
    
    Dim nBlockAlign As Integer
    Dim wBits As Integer
    Dim nChannels As Integer
    
    Dim vL As Single
    Dim vR As Single
    Dim dist As Single
    Dim lp As Single
    Dim hp As Single
    Dim ptch As Double
    
    Dim sWidth As Single
    Dim trmRate As Single
    Dim trmDepth As Single
    Dim trmPhase As Double
    Dim trmStep As Double
    
    Dim cDepth As Single
    Dim cRate As Single
    Dim cPhase As Double
    Dim cStep As Double
    
    Dim rmFreq As Single
    Dim rmMix As Single
    Dim rmPhase As Double
    Dim rmStep As Double
    
    Dim apRate As Single
    Dim apDepth As Single
    Dim apPhase As Double
    Dim apStep As Double
    
    Dim flgRate As Single
    Dim flgDepth As Single
    Dim flgFB As Single
    Dim flgPhase As Double
    Dim flgStep As Double
    
    Dim eqB As Single
    Dim eqM As Single
    Dim eqT As Single
    Dim eqAlphaLow As Single
    Dim eqAlphaHigh As Single
    
    Dim cmpThresh As Single
    Dim cmpRatio As Single
    Dim cmpEnv As Single
    Dim cmpGain As Single
    
    Dim bdSteps As Single
    Dim dsFactor As Long
    Dim dsCount As Long
    Dim lastL As Single
    Dim lastR As Single
    
    Dim rMix As Single
    Dim rTime As Single
    Dim rt1 As Long
    Dim rt2 As Long
    Dim rt3 As Long
    Dim rt4 As Long
    
    Dim dTime As Single
    Dim dFB As Single
    Dim dMix As Single
    Dim dSamples As Long
    Dim dWrite As Long
    Dim dBase As Long
    
    Dim align As Long
    Dim ptchAlign As Double
    Dim loopSnd As Boolean
    Dim pos As Double
    Dim loopStart As Double
    Dim loopEnd As Double
    
    Dim fadeState As Long
    Dim fadeCur As Long
    Dim fadeTot As Long
    Dim fadeMult As Single
    
    Dim fL As Single
    Dim fR As Single
    Dim l1 As Long
    Dim l2 As Long
    
    Dim srcIdx As Double
    Dim writeIdx As Long
    
    Dim framesNeeded As Long
    Dim bytesNeeded As Long
    Dim bytesAvail As Long
    Dim remBytes As Long
    
    #If VBA7 Then
        Dim pData As LongPtr
        Dim ptr As LongPtr
    #Else
        Dim pData As Long
        Dim ptr As Long
    #End If
    
    hr = vCall(rCtx.AudioClient, 6, VarPtr(padding))
    If hr <> 0 Then
        Exit Sub
    End If
    
    framesAvailable = rCtx.BufferSize - padding
    If framesAvailable <= 0 Then
        Exit Sub
    End If
    
    hr = vCall(rCtx.RenderClient, 3, framesAvailable, VarPtr(pData))
    If hr <> 0 Or pData = 0 Then
        Exit Sub
    End If
    
    RtlMoveMemory VarPtr(nChannels), ByVal (rCtx.MixFormatPtr + 2), 2
    RtlMoveMemory VarPtr(nBlockAlign), ByVal (rCtx.MixFormatPtr + 12), 2
    RtlMoveMemory VarPtr(wBits), ByVal (rCtx.MixFormatPtr + 14), 2
    
    bytesToWrite = framesAvailable * CLng(nBlockAlign)
    align = CLng(nBlockAlign)
    
    eqAlphaLow = 1! - Exp(-PI2 * 200! / CSng(rCtx.sampleRate))
    eqAlphaHigh = 1! - Exp(-PI2 * 2000! / CSng(rCtx.sampleRate))
    
    rCtx.MasterPeakL = rCtx.MasterPeakL * 0.9!
    rCtx.MasterPeakR = rCtx.MasterPeakR * 0.9!
    
    Dim currentMasterPeakL As Single
    Dim currentMasterPeakR As Single
    currentMasterPeakL = 0!
    currentMasterPeakR = 0!
    
    If wBits = 32 Then
        Dim mixArr32() As Single
        ReDim mixArr32(0 To (bytesToWrite \ 4) - 1)
        
        For i = 0 To 31
            If rVoices(i).Active And rVoices(i).Playing Then
                
                If rVoices(i).Paused Then
                    GoTo NextVoice32
                End If
                
                Dim isSourceValid As Boolean
                isSourceValid = False
                
                If rVoices(i).IsOscillator Then
                    isSourceValid = True
                ElseIf rVoices(i).BufferIndex >= 0 Then
                    If rCtx.Buffers(rVoices(i).BufferIndex).Active Then
                        isSourceValid = True
                    End If
                End If
                
                If isSourceValid Then
                    
                    pos = rVoices(i).Position
                    ptch = rVoices(i).Pitch
                    loopSnd = rVoices(i).Looping
                    loopStart = rVoices(i).loopStart
                    loopEnd = rVoices(i).loopEnd
                    
                    Dim srcArr32() As Single
                    framesNeeded = Int(framesAvailable * ptch) + 2
                    bytesNeeded = framesNeeded * align
                    
                    If Not rVoices(i).IsOscillator Then
                        
                        ptr = rCtx.Buffers(rVoices(i).BufferIndex).BufferPtr
                        readPos = (CLng(pos) \ align) * align
                        
                        If readPos < 0 Then
                            readPos = 0
                        End If
                        
                        bytesAvail = CLng(loopEnd) - readPos
                        If bytesAvail < 0 Then
                            bytesAvail = 0
                        End If
                        
                        ReDim srcArr32(0 To (bytesNeeded \ 4) - 1)
                        
                        If bytesNeeded <= bytesAvail Then
                            RtlMoveMemoryToSingle srcArr32(0), ByVal (ptr + readPos), bytesNeeded
                        Else
                            If bytesAvail > 0 Then
                                RtlMoveMemoryToSingle srcArr32(0), ByVal (ptr + readPos), bytesAvail
                            End If
                            If loopSnd Then
                                remBytes = bytesNeeded - bytesAvail
                                If remBytes > CLng(loopEnd - loopStart) Then
                                    remBytes = CLng(loopEnd - loopStart)
                                End If
                                If remBytes > 0 Then
                                    RtlMoveMemoryToSingle srcArr32(bytesAvail \ 4), ByVal (ptr + CLng(loopStart)), remBytes
                                End If
                            End If
                        End If
                        
                    End If
                    
                    dist = rVoices(i).Distortion
                    lp = rVoices(i).lowPass
                    hp = rVoices(i).highPass
                    sWidth = rVoices(i).StereoWidth
                    
                    eqB = rVoices(i).EqBass
                    eqM = rVoices(i).EqMid
                    eqT = rVoices(i).EqTreble
                    
                    cmpThresh = rVoices(i).CompThreshold
                    cmpRatio = rVoices(i).CompRatio
                    cmpEnv = rVoices(i).CompEnv
                    
                    bdSteps = rVoices(i).BitcrushSteps
                    dsFactor = rVoices(i).BitcrushDownsample
                    dsCount = rVoices(i).BitcrushDsCount
                    lastL = rVoices(i).BitcrushLastL
                    lastR = rVoices(i).BitcrushLastR
                    
                    rmFreq = rVoices(i).RingModFreq
                    rmMix = rVoices(i).RingModMix
                    rmPhase = rVoices(i).RingModPhase
                    rmStep = (PI2 * rmFreq) / CDbl(rCtx.sampleRate)
                    
                    trmRate = rVoices(i).TremoloRate
                    trmDepth = rVoices(i).TremoloDepth
                    trmPhase = rVoices(i).TremoloPhase
                    trmStep = (PI2 * trmRate) / CDbl(rCtx.sampleRate)
                    
                    apRate = rVoices(i).AutoPanRate
                    apDepth = rVoices(i).AutoPanDepth
                    apPhase = rVoices(i).AutoPanPhase
                    apStep = (PI2 * apRate) / CDbl(rCtx.sampleRate)
                    
                    cRate = rVoices(i).ChorusRate
                    cDepth = rVoices(i).ChorusDepth
                    cPhase = rVoices(i).ChorusPhase
                    cStep = (PI2 * cRate) / CDbl(rCtx.sampleRate)
                    
                    flgRate = rVoices(i).FlangerRate
                    flgDepth = rVoices(i).FlangerDepth
                    flgFB = rVoices(i).FlangerFeedback
                    flgPhase = rVoices(i).FlangerPhase
                    flgStep = (PI2 * flgRate) / CDbl(rCtx.sampleRate)
                    
                    rMix = rVoices(i).ReverbMix
                    rTime = rVoices(i).ReverbTime
                    rt1 = rVoices(i).RevTap1
                    rt2 = rVoices(i).RevTap2
                    rt3 = rVoices(i).RevTap3
                    rt4 = rVoices(i).RevTap4
                    
                    dTime = rVoices(i).DelayTime
                    dFB = rVoices(i).DelayFeedback
                    dMix = rVoices(i).DelayMix
                    dSamples = Int(dTime * rCtx.sampleRate) * 2
                    dWrite = rVoices(i).RingWritePos
                    dBase = i * 192000
                    
                    fadeState = rVoices(i).fadeState
                    fadeCur = rVoices(i).FadeFramesCurrent
                    fadeTot = rVoices(i).FadeFramesTotal
                    
                    srcIdx = 0
                    writeIdx = 0
                    ptchAlign = ptch * CDbl(align)
                    
                    Dim currentVoicePeakL As Single
                    Dim currentVoicePeakR As Single
                    currentVoicePeakL = 0!
                    currentVoicePeakR = 0!
                    rVoices(i).PeakL = rVoices(i).PeakL * 0.9!
                    rVoices(i).PeakR = rVoices(i).PeakR * 0.9!
                    
                    If nChannels = 2 Then
                        For frame = 0 To framesAvailable - 1
                            If Not rVoices(i).IsOscillator Then
                                If pos >= loopEnd Then
                                    If loopSnd Then
                                        pos = loopStart
                                    Else
                                        rVoices(i).Playing = False
                                        rVoices(i).Active = False
                                        Exit For
                                    End If
                                End If
                            End If
                            
                            If fadeState = 1 Then
                                If fadeCur < fadeTot Then
                                    fadeCur = fadeCur + 1
                                    fadeMult = CSng(fadeCur) / CSng(fadeTot)
                                Else
                                    fadeState = 0
                                    fadeMult = 1!
                                End If
                            ElseIf fadeState = 2 Then
                                If fadeCur < fadeTot Then
                                    fadeCur = fadeCur + 1
                                    fadeMult = 1! - (CSng(fadeCur) / CSng(fadeTot))
                                Else
                                    rVoices(i).Playing = False
                                    rVoices(i).Active = False
                                    Exit For
                                End If
                            Else
                                fadeMult = 1!
                            End If
                            
                            If rVoices(i).IsOscillator Then
                                fL = RiffNextOscillatorSample(i)
                                fR = fL
                            Else
                                Dim sID As Long
                                sID = Int(srcIdx) * 2
                                If sID + 1 <= UBound(srcArr32) Then
                                    fL = srcArr32(sID)
                                    fR = srcArr32(sID + 1)
                                Else
                                    fL = 0!
                                    fR = 0!
                                End If
                            End If
                            
                            If dsFactor > 1 Then
                                If dsCount >= dsFactor Then
                                    dsCount = 0
                                    lastL = fL
                                    lastR = fR
                                Else
                                    fL = lastL
                                    fR = lastR
                                    dsCount = dsCount + 1
                                End If
                            End If
                            
                            If bdSteps > 0! Then
                                fL = Fix(fL * bdSteps) / bdSteps
                                fR = Fix(fR * bdSteps) / bdSteps
                            End If
                            
                            fL = fL * dist
                            If fL > 1! Then
                                fL = 1!
                            ElseIf fL < -1! Then
                                fL = -1!
                            End If
                            
                            fR = fR * dist
                            If fR > 1! Then
                                fR = 1!
                            ElseIf fR < -1! Then
                                fR = -1!
                            End If
                            
                            RiffProcessVoiceFilters i, fL, fR, lp, hp, eqB, eqM, eqT
                            
                            If rmMix > 0! Then
                                Dim rmOsc As Single
                                rmOsc = Sin(rmPhase)
                                fL = fL * (1! - rmMix) + (fL * rmOsc) * rmMix
                                fR = fR * (1! - rmMix) + (fR * rmOsc) * rmMix
                                rmPhase = rmPhase + rmStep
                                If rmPhase > PI2 Then
                                    rmPhase = rmPhase - PI2
                                End If
                            End If
                            
                            If trmDepth > 0! Then
                                Dim trmMult As Single
                                trmMult = 1! - trmDepth * (0.5! + 0.5! * CSng(Sin(trmPhase)))
                                fL = fL * trmMult
                                fR = fR * trmMult
                                trmPhase = trmPhase + trmStep
                                If trmPhase > PI2 Then
                                    trmPhase = trmPhase - PI2
                                End If
                            End If
                            
                            If sWidth <> 1! Then
                                Dim midS As Single
                                Dim sideS As Single
                                midS = (fL + fR) * 0.5!
                                sideS = (fL - fR) * 0.5!
                                fL = midS + sideS * sWidth
                                fR = midS - sideS * sWidth
                            End If
                            
                            Dim bufInL As Single
                            Dim bufInR As Single
                            bufInL = fL
                            bufInR = fR
                            
                            If flgDepth > 0! Then
                                Dim fDel As Long
                                Dim fRd As Long
                                Dim flgL As Single
                                Dim flgR As Single
                                
                                fDel = Int((0.002! + 0.005! * CSng(Sin(flgPhase))) * rCtx.sampleRate) * 2
                                fRd = dWrite - fDel
                                If fRd < 0 Then
                                    fRd = fRd + 192000
                                End If
                                
                                flgL = rRingBuf(dBase + fRd)
                                flgR = rRingBuf(dBase + fRd + 1)
                                
                                fL = fL + flgL * flgDepth
                                fR = fR + flgR * flgDepth
                                bufInL = bufInL + flgL * flgFB
                                bufInR = bufInR + flgR * flgFB
                                
                                flgPhase = flgPhase + flgStep
                                If flgPhase > PI2 Then
                                    flgPhase = flgPhase - PI2
                                End If
                            End If
                            
                            If cDepth > 0! Then
                                Dim cDelay As Long
                                Dim cRead As Long
                                
                                cDelay = Int((0.02! + 0.005! * CSng(Sin(cPhase))) * rCtx.sampleRate) * 2
                                cRead = dWrite - cDelay
                                If cRead < 0 Then
                                    cRead = cRead + 192000
                                End If
                                
                                fL = fL * (1! - cDepth * 0.5!) + rRingBuf(dBase + cRead) * cDepth
                                fR = fR * (1! - cDepth * 0.5!) + rRingBuf(dBase + cRead + 1) * cDepth
                                cPhase = cPhase + cStep
                                If cPhase > PI2 Then
                                    cPhase = cPhase - PI2
                                End If
                            End If
                            
                            If dMix > 0! And dSamples > 0 Then
                                Dim dRead As Long
                                Dim dL As Single
                                Dim dR As Single
                                
                                dRead = dWrite - dSamples
                                If dRead < 0 Then
                                    dRead = dRead + 192000
                                End If
                                
                                dL = rRingBuf(dBase + dRead)
                                dR = rRingBuf(dBase + dRead + 1)
                                fL = fL + dL * dMix
                                fR = fR + dR * dMix
                                bufInL = bufInL + dL * dFB
                                bufInR = bufInR + dR * dFB
                            End If
                            
                            If rMix > 0! Then
                                RiffProcessFreeverb i, dBase, dWrite, rMix, rTime, fL, fR, bufInL, bufInR
                            End If
                            
                            rRingBuf(dBase + dWrite) = bufInL
                            rRingBuf(dBase + dWrite + 1) = bufInR
                            dWrite = dWrite + 2
                            If dWrite >= 192000 Then
                                dWrite = 0
                            End If
                            
                            If cmpRatio > 1! Then
                                Dim pkL As Single
                                Dim pkR As Single
                                Dim maxPk As Single
                                
                                pkL = Abs(fL)
                                pkR = Abs(fR)
                                maxPk = pkL
                                If pkR > maxPk Then
                                    maxPk = pkR
                                End If
                                
                                If maxPk > cmpEnv Then
                                    cmpEnv = cmpEnv + 0.01! * (maxPk - cmpEnv)
                                Else
                                    cmpEnv = cmpEnv + 0.001! * (maxPk - cmpEnv)
                                End If
                                
                                If cmpEnv > cmpThresh And cmpEnv > 0.0001! Then
                                    cmpGain = cmpThresh + ((cmpEnv - cmpThresh) / cmpRatio)
                                    cmpGain = cmpGain / cmpEnv
                                    fL = fL * cmpGain
                                    fR = fR * cmpGain
                                End If
                            End If
                            
                            Dim curPan As Single
                            curPan = rVoices(i).Pan
                            If apDepth > 0! Then
                                curPan = curPan + (Sin(apPhase) * apDepth)
                                If curPan > 1! Then
                                    curPan = 1!
                                ElseIf curPan < -1! Then
                                    curPan = -1!
                                End If
                                apPhase = apPhase + apStep
                                If apPhase > PI2 Then
                                    apPhase = apPhase - PI2
                                End If
                            End If
                            
                            Dim busVol As Single
                            busVol = rCtx.Buses(rVoices(i).busID)
                            
                            vL = rVoices(i).Volume * rCtx.MasterVolume * busVol
                            vR = rVoices(i).Volume * rCtx.MasterVolume * busVol
                            
                            If curPan > 0! Then
                                vL = vL * (1! - curPan)
                            End If
                            If curPan < 0! Then
                                vR = vR * (1! + curPan)
                            End If
                            
                            fL = fL * vL * fadeMult
                            fR = fR * vR * fadeMult
                            
                            If Abs(fL) > currentVoicePeakL Then
                                currentVoicePeakL = Abs(fL)
                            End If
                            If Abs(fR) > currentVoicePeakR Then
                                currentVoicePeakR = Abs(fR)
                            End If
                            
                            mixArr32(writeIdx) = mixArr32(writeIdx) + fL
                            mixArr32(writeIdx + 1) = mixArr32(writeIdx + 1) + fR
                            
                            If Abs(mixArr32(writeIdx)) > currentMasterPeakL Then
                                currentMasterPeakL = Abs(mixArr32(writeIdx))
                            End If
                            If Abs(mixArr32(writeIdx + 1)) > currentMasterPeakR Then
                                currentMasterPeakR = Abs(mixArr32(writeIdx + 1))
                            End If
                            
                            writeIdx = writeIdx + 2
                            srcIdx = srcIdx + ptch
                            pos = pos + ptchAlign
                        Next frame
                    End If
                    
                    If currentVoicePeakL > rVoices(i).PeakL Then
                        rVoices(i).PeakL = currentVoicePeakL
                    End If
                    If currentVoicePeakR > rVoices(i).PeakR Then
                        rVoices(i).PeakR = currentVoicePeakR
                    End If
                    
                    rVoices(i).Position = pos
                    rVoices(i).TremoloPhase = trmPhase
                    rVoices(i).AutoPanPhase = apPhase
                    rVoices(i).RingModPhase = rmPhase
                    rVoices(i).ChorusPhase = cPhase
                    rVoices(i).FlangerPhase = flgPhase
                    rVoices(i).RingWritePos = dWrite
                    rVoices(i).CompEnv = cmpEnv
                    rVoices(i).BitcrushDsCount = dsCount
                    rVoices(i).BitcrushLastL = lastL
                    rVoices(i).BitcrushLastR = lastR
                    rVoices(i).fadeState = fadeState
                    rVoices(i).FadeFramesCurrent = fadeCur
                End If
            End If
NextVoice32:
        Next i
        
        If currentMasterPeakL > rCtx.MasterPeakL Then
            rCtx.MasterPeakL = currentMasterPeakL
        End If
        If currentMasterPeakR > rCtx.MasterPeakR Then
            rCtx.MasterPeakR = currentMasterPeakR
        End If
        
        For frame = 0 To (bytesToWrite \ 4) - 1
            If mixArr32(frame) > 1! Then
                mixArr32(frame) = 1!
            ElseIf mixArr32(frame) < -1! Then
                mixArr32(frame) = -1!
            End If
        Next frame
        
        If RiffMixFormatIsFloat32() Then
            RtlMoveMemory ByVal pData, VarPtr(mixArr32(0)), bytesToWrite
        Else
            Dim mixInt32() As Long
            ReDim mixInt32(0 To (bytesToWrite \ 4) - 1)
            For frame = 0 To UBound(mixArr32)
                If mixArr32(frame) >= 1! Then
                    mixInt32(frame) = 2147483647
                ElseIf mixArr32(frame) <= -1! Then
                    mixInt32(frame) = -2147483647
                Else
                    mixInt32(frame) = CLng(mixArr32(frame) * 2147483647#)
                End If
            Next frame
            RtlMoveMemory ByVal pData, VarPtr(mixInt32(0)), bytesToWrite
        End If

    ElseIf wBits = 16 Then
        Dim mixArr16() As Integer
        ReDim mixArr16(0 To (bytesToWrite \ 2) - 1)
        
        For i = 0 To 31
            If rVoices(i).Active And rVoices(i).Playing Then
                
                If rVoices(i).Paused Then
                    GoTo NextVoice16
                End If
                
                Dim isSourceValid16 As Boolean
                isSourceValid16 = False
                
                If rVoices(i).IsOscillator Then
                    isSourceValid16 = True
                ElseIf rVoices(i).BufferIndex >= 0 Then
                    If rCtx.Buffers(rVoices(i).BufferIndex).Active Then
                        isSourceValid16 = True
                    End If
                End If
                
                If isSourceValid16 Then
                    
                    pos = rVoices(i).Position
                    ptch = rVoices(i).Pitch
                    loopSnd = rVoices(i).Looping
                    loopStart = rVoices(i).loopStart
                    loopEnd = rVoices(i).loopEnd
                    
                    Dim srcArr16() As Integer
                    framesNeeded = Int(framesAvailable * ptch) + 2
                    bytesNeeded = framesNeeded * align
                    
                    If Not rVoices(i).IsOscillator Then
                        
                        ptr = rCtx.Buffers(rVoices(i).BufferIndex).BufferPtr
                        readPos = (CLng(pos) \ align) * align
                        
                        If readPos < 0 Then
                            readPos = 0
                        End If
                        
                        bytesAvail = CLng(loopEnd) - readPos
                        If bytesAvail < 0 Then
                            bytesAvail = 0
                        End If
                        
                        ReDim srcArr16(0 To (bytesNeeded \ 2) - 1)
                        
                        If bytesNeeded <= bytesAvail Then
                            RtlMoveMemoryToInteger srcArr16(0), ByVal (ptr + readPos), bytesNeeded
                        Else
                            If bytesAvail > 0 Then
                                RtlMoveMemoryToInteger srcArr16(0), ByVal (ptr + readPos), bytesAvail
                            End If
                            If loopSnd Then
                                remBytes = bytesNeeded - bytesAvail
                                If remBytes > CLng(loopEnd - loopStart) Then
                                    remBytes = CLng(loopEnd - loopStart)
                                End If
                                If remBytes > 0 Then
                                    RtlMoveMemoryToInteger srcArr16(bytesAvail \ 2), ByVal (ptr + CLng(loopStart)), remBytes
                                End If
                            End If
                        End If
                        
                    End If
                    
                    dist = rVoices(i).Distortion
                    lp = rVoices(i).lowPass
                    hp = rVoices(i).highPass
                    sWidth = rVoices(i).StereoWidth
                    
                    eqB = rVoices(i).EqBass
                    eqM = rVoices(i).EqMid
                    eqT = rVoices(i).EqTreble
                    
                    cmpThresh = rVoices(i).CompThreshold
                    cmpRatio = rVoices(i).CompRatio
                    cmpEnv = rVoices(i).CompEnv
                    
                    bdSteps = rVoices(i).BitcrushSteps
                    dsFactor = rVoices(i).BitcrushDownsample
                    dsCount = rVoices(i).BitcrushDsCount
                    lastL = rVoices(i).BitcrushLastL
                    lastR = rVoices(i).BitcrushLastR
                    
                    rmFreq = rVoices(i).RingModFreq
                    rmMix = rVoices(i).RingModMix
                    rmPhase = rVoices(i).RingModPhase
                    rmStep = (PI2 * rmFreq) / CDbl(rCtx.sampleRate)
                    
                    trmRate = rVoices(i).TremoloRate
                    trmDepth = rVoices(i).TremoloDepth
                    trmPhase = rVoices(i).TremoloPhase
                    trmStep = (PI2 * trmRate) / CDbl(rCtx.sampleRate)
                    
                    apRate = rVoices(i).AutoPanRate
                    apDepth = rVoices(i).AutoPanDepth
                    apPhase = rVoices(i).AutoPanPhase
                    apStep = (PI2 * apRate) / CDbl(rCtx.sampleRate)
                    
                    cRate = rVoices(i).ChorusRate
                    cDepth = rVoices(i).ChorusDepth
                    cPhase = rVoices(i).ChorusPhase
                    cStep = (PI2 * cRate) / CDbl(rCtx.sampleRate)
                    
                    flgRate = rVoices(i).FlangerRate
                    flgDepth = rVoices(i).FlangerDepth
                    flgFB = rVoices(i).FlangerFeedback
                    flgPhase = rVoices(i).FlangerPhase
                    flgStep = (PI2 * flgRate) / CDbl(rCtx.sampleRate)
                    
                    rMix = rVoices(i).ReverbMix
                    rTime = rVoices(i).ReverbTime
                    rt1 = rVoices(i).RevTap1
                    rt2 = rVoices(i).RevTap2
                    rt3 = rVoices(i).RevTap3
                    rt4 = rVoices(i).RevTap4
                    
                    dTime = rVoices(i).DelayTime
                    dFB = rVoices(i).DelayFeedback
                    dMix = rVoices(i).DelayMix
                    dSamples = Int(dTime * rCtx.sampleRate) * 2
                    dWrite = rVoices(i).RingWritePos
                    dBase = i * 192000
                    
                    fadeState = rVoices(i).fadeState
                    fadeCur = rVoices(i).FadeFramesCurrent
                    fadeTot = rVoices(i).FadeFramesTotal
                    
                    srcIdx = 0
                    writeIdx = 0
                    ptchAlign = ptch * CDbl(align)
                    
                    Dim cVoicePeakL16 As Single
                    Dim cVoicePeakR16 As Single
                    cVoicePeakL16 = 0!
                    cVoicePeakR16 = 0!
                    rVoices(i).PeakL = rVoices(i).PeakL * 0.9!
                    rVoices(i).PeakR = rVoices(i).PeakR * 0.9!
                    
                    If nChannels = 2 Then
                        For frame = 0 To framesAvailable - 1
                            If Not rVoices(i).IsOscillator Then
                                If pos >= loopEnd Then
                                    If loopSnd Then
                                        pos = loopStart
                                    Else
                                        rVoices(i).Playing = False
                                        rVoices(i).Active = False
                                        Exit For
                                    End If
                                End If
                            End If
                            
                            If fadeState = 1 Then
                                If fadeCur < fadeTot Then
                                    fadeCur = fadeCur + 1
                                    fadeMult = CSng(fadeCur) / CSng(fadeTot)
                                Else
                                    fadeState = 0
                                    fadeMult = 1!
                                End If
                            ElseIf fadeState = 2 Then
                                If fadeCur < fadeTot Then
                                    fadeCur = fadeCur + 1
                                    fadeMult = 1! - (CSng(fadeCur) / CSng(fadeTot))
                                Else
                                    rVoices(i).Playing = False
                                    rVoices(i).Active = False
                                    Exit For
                                End If
                            Else
                                fadeMult = 1!
                            End If
                            
                            If rVoices(i).IsOscillator Then
                                fL = RiffNextOscillatorSample(i)
                                fR = fL
                            Else
                                Dim sID16 As Long
                                sID16 = Int(srcIdx) * 2
                                If sID16 + 1 <= UBound(srcArr16) Then
                                    fL = CSng(srcArr16(sID16)) * 3.051758E-05!
                                    fR = CSng(srcArr16(sID16 + 1)) * 3.051758E-05!
                                Else
                                    fL = 0!
                                    fR = 0!
                                End If
                            End If
                            
                            If dsFactor > 1 Then
                                If dsCount >= dsFactor Then
                                    dsCount = 0
                                    lastL = fL
                                    lastR = fR
                                Else
                                    fL = lastL
                                    fR = lastR
                                    dsCount = dsCount + 1
                                End If
                            End If
                            
                            If bdSteps > 0! Then
                                fL = Fix(fL * bdSteps) / bdSteps
                                fR = Fix(fR * bdSteps) / bdSteps
                            End If
                            
                            fL = fL * dist
                            If fL > 1! Then
                                fL = 1!
                            ElseIf fL < -1! Then
                                fL = -1!
                            End If
                            
                            fR = fR * dist
                            If fR > 1! Then
                                fR = 1!
                            ElseIf fR < -1! Then
                                fR = -1!
                            End If
                            
                            RiffProcessVoiceFilters i, fL, fR, lp, hp, eqB, eqM, eqT
                            
                            If rmMix > 0! Then
                                Dim rmOsc16 As Single
                                rmOsc16 = Sin(rmPhase)
                                fL = fL * (1! - rmMix) + (fL * rmOsc16) * rmMix
                                fR = fR * (1! - rmMix) + (fR * rmOsc16) * rmMix
                                rmPhase = rmPhase + rmStep
                                If rmPhase > PI2 Then
                                    rmPhase = rmPhase - PI2
                                End If
                            End If
                            
                            If trmDepth > 0! Then
                                Dim trmM16 As Single
                                trmM16 = 1! - trmDepth * (0.5! + 0.5! * CSng(Sin(trmPhase)))
                                fL = fL * trmM16
                                fR = fR * trmM16
                                trmPhase = trmPhase + trmStep
                                If trmPhase > PI2 Then
                                    trmPhase = trmPhase - PI2
                                End If
                            End If
                            
                            If sWidth <> 1! Then
                                Dim m16 As Single
                                Dim sd16 As Single
                                m16 = (fL + fR) * 0.5!
                                sd16 = (fL - fR) * 0.5!
                                fL = m16 + sd16 * sWidth
                                fR = m16 - sd16 * sWidth
                            End If
                            
                            Dim bufInL16 As Single
                            Dim bufInR16 As Single
                            bufInL16 = fL
                            bufInR16 = fR
                            
                            If flgDepth > 0! Then
                                Dim fDel16 As Long
                                Dim fRd16 As Long
                                Dim flgL16 As Single
                                Dim flgR16 As Single
                                
                                fDel16 = Int((0.002! + 0.005! * CSng(Sin(flgPhase))) * rCtx.sampleRate) * 2
                                fRd16 = dWrite - fDel16
                                If fRd16 < 0 Then
                                    fRd16 = fRd16 + 192000
                                End If
                                
                                flgL16 = rRingBuf(dBase + fRd16)
                                flgR16 = rRingBuf(dBase + fRd16 + 1)
                                
                                fL = fL + flgL16 * flgDepth
                                fR = fR + flgR16 * flgDepth
                                bufInL16 = bufInL16 + flgL16 * flgFB
                                bufInR16 = bufInR16 + flgR16 * flgFB
                                
                                flgPhase = flgPhase + flgStep
                                If flgPhase > PI2 Then
                                    flgPhase = flgPhase - PI2
                                End If
                            End If
                            
                            If cDepth > 0! Then
                                Dim cDel16 As Long
                                Dim cRd16 As Long
                                
                                cDel16 = Int((0.02! + 0.005! * CSng(Sin(cPhase))) * rCtx.sampleRate) * 2
                                cRd16 = dWrite - cDel16
                                If cRd16 < 0 Then
                                    cRd16 = cRd16 + 192000
                                End If
                                
                                fL = fL * (1! - cDepth * 0.5!) + rRingBuf(dBase + cRd16) * cDepth
                                fR = fR * (1! - cDepth * 0.5!) + rRingBuf(dBase + cRd16 + 1) * cDepth
                                cPhase = cPhase + cStep
                                If cPhase > PI2 Then
                                    cPhase = cPhase - PI2
                                End If
                            End If
                            
                            If dMix > 0! And dSamples > 0 Then
                                Dim dR16 As Long
                                Dim dL16 As Single
                                Dim dR16_2 As Single
                                
                                dR16 = dWrite - dSamples
                                If dR16 < 0 Then
                                    dR16 = dR16 + 192000
                                End If
                                
                                dL16 = rRingBuf(dBase + dR16)
                                dR16_2 = rRingBuf(dBase + dR16 + 1)
                                fL = fL + dL16 * dMix
                                fR = fR + dR16_2 * dMix
                                bufInL16 = bufInL16 + dL16 * dFB
                                bufInR16 = bufInR16 + dR16_2 * dFB
                            End If
                            
                            If rMix > 0! Then
                                RiffProcessFreeverb i, dBase, dWrite, rMix, rTime, fL, fR, bufInL16, bufInR16
                            End If
                            
                            rRingBuf(dBase + dWrite) = bufInL16
                            rRingBuf(dBase + dWrite + 1) = bufInR16
                            dWrite = dWrite + 2
                            If dWrite >= 192000 Then
                                dWrite = 0
                            End If
                            
                            If cmpRatio > 1! Then
                                Dim pkL16 As Single
                                Dim pkR16 As Single
                                Dim maxPk16 As Single
                                
                                pkL16 = Abs(fL)
                                pkR16 = Abs(fR)
                                maxPk16 = pkL16
                                If pkR16 > maxPk16 Then
                                    maxPk16 = pkR16
                                End If
                                
                                If maxPk16 > cmpEnv Then
                                    cmpEnv = cmpEnv + 0.01! * (maxPk16 - cmpEnv)
                                Else
                                    cmpEnv = cmpEnv + 0.001! * (maxPk16 - cmpEnv)
                                End If
                                
                                If cmpEnv > cmpThresh And cmpEnv > 0.0001! Then
                                    cmpGain = cmpThresh + ((cmpEnv - cmpThresh) / cmpRatio)
                                    cmpGain = cmpGain / cmpEnv
                                    fL = fL * cmpGain
                                    fR = fR * cmpGain
                                End If
                            End If
                            
                            Dim cPan16 As Single
                            cPan16 = rVoices(i).Pan
                            If apDepth > 0! Then
                                cPan16 = cPan16 + (Sin(apPhase) * apDepth)
                                If cPan16 > 1! Then
                                    cPan16 = 1!
                                ElseIf cPan16 < -1! Then
                                    cPan16 = -1!
                                End If
                                apPhase = apPhase + apStep
                                If apPhase > PI2 Then
                                    apPhase = apPhase - PI2
                                End If
                            End If
                            
                            Dim bVol16 As Single
                            bVol16 = rCtx.Buses(rVoices(i).busID)
                            
                            vL = rVoices(i).Volume * rCtx.MasterVolume * bVol16
                            vR = rVoices(i).Volume * rCtx.MasterVolume * bVol16
                            
                            If cPan16 > 0! Then
                                vL = vL * (1! - cPan16)
                            End If
                            If cPan16 < 0! Then
                                vR = vR * (1! + cPan16)
                            End If
                            
                            fL = fL * vL * fadeMult
                            fR = fR * vR * fadeMult
                            
                            If Abs(fL) > cVoicePeakL16 Then
                                cVoicePeakL16 = Abs(fL)
                            End If
                            If Abs(fR) > cVoicePeakR16 Then
                                cVoicePeakR16 = Abs(fR)
                            End If
                            
                            l1 = CLng(mixArr16(writeIdx)) + CLng(fL * 32767!)
                            l2 = CLng(mixArr16(writeIdx + 1)) + CLng(fR * 32767!)
                            
                            If l1 > 32767 Then
                                l1 = 32767
                            ElseIf l1 < -32768 Then
                                l1 = -32768
                            End If
                            
                            If l2 > 32767 Then
                                l2 = 32767
                            ElseIf l2 < -32768 Then
                                l2 = -32768
                            End If
                            
                            mixArr16(writeIdx) = CInt(l1)
                            mixArr16(writeIdx + 1) = CInt(l2)
                            
                            If Abs(fL) > currentMasterPeakL Then
                                currentMasterPeakL = Abs(fL)
                            End If
                            If Abs(fR) > currentMasterPeakR Then
                                currentMasterPeakR = Abs(fR)
                            End If
                            
                            writeIdx = writeIdx + 2
                            srcIdx = srcIdx + ptch
                            pos = pos + ptchAlign
                        Next frame
                    End If
                    
                    If cVoicePeakL16 > rVoices(i).PeakL Then
                        rVoices(i).PeakL = cVoicePeakL16
                    End If
                    If cVoicePeakR16 > rVoices(i).PeakR Then
                        rVoices(i).PeakR = cVoicePeakR16
                    End If
                    
                    rVoices(i).Position = pos
                    rVoices(i).TremoloPhase = trmPhase
                    rVoices(i).AutoPanPhase = apPhase
                    rVoices(i).RingModPhase = rmPhase
                    rVoices(i).ChorusPhase = cPhase
                    rVoices(i).FlangerPhase = flgPhase
                    rVoices(i).RingWritePos = dWrite
                    rVoices(i).CompEnv = cmpEnv
                    rVoices(i).BitcrushDsCount = dsCount
                    rVoices(i).BitcrushLastL = lastL
                    rVoices(i).BitcrushLastR = lastR
                    rVoices(i).fadeState = fadeState
                    rVoices(i).FadeFramesCurrent = fadeCur
                End If
            End If
NextVoice16:
        Next i
        
        If currentMasterPeakL > rCtx.MasterPeakL Then
            rCtx.MasterPeakL = currentMasterPeakL
        End If
        If currentMasterPeakR > rCtx.MasterPeakR Then
            rCtx.MasterPeakR = currentMasterPeakR
        End If
        
        RtlMoveMemory ByVal pData, VarPtr(mixArr16(0)), bytesToWrite
    Else
        RtlZeroMemory pData, bytesToWrite
    End If
    
    vCall rCtx.RenderClient, 4, framesAvailable, 0&
End Sub

'/**
' * @function DummyEbMode
' * @brief Safe fallback function if EbMode is inaccessible.
' * @return {Long} Always returns 1.
' */
Private Function DummyEbMode() As Long
    DummyEbMode = 1
End Function

'/**
' * @function GetAddressOf
' * @brief Extracts the memory address pointer of a VBA AddressOf callback safely.
' * @param ptr Function pointer.
' * @return {Long/LongPtr} The raw address.
' */
#If VBA7 Then
Private Function GetAddressOf(ByVal ptr As LongPtr) As LongPtr
    GetAddressOf = ptr
End Function
#Else
Private Function GetAddressOf(ByVal ptr As Long) As Long
    GetAddressOf = ptr
End Function
#End If

'/**
' * @function InitThunks
' * @brief Compiles machine code at runtime to bridge Win32 Timers and VBA logic.
' * @return {Boolean} True if successful.
' */
Private Function InitThunks() As Boolean
    #If VBA7 Then
        Const THUNK_SIZE As Long = 1024
    #Else
        Const THUNK_SIZE As Long = 512
    #End If
    
    rCtx.ThunkTimerCB = VirtualAlloc(0, THUNK_SIZE, MEM_COMMIT Or MEM_RESERVE, PAGE_EXECUTE_READWRITE)
    If rCtx.ThunkTimerCB = 0 Then
        Exit Function
    End If
    
    Dim opcodes() As Byte
    Dim hexStr As String
    Dim i As Long
    
    #If VBA7 Then
        Dim hVbe As LongPtr
        Dim pEbMode As LongPtr
        Dim pCallback As LongPtr
        Dim pKill As LongPtr
    #Else
        Dim hVbe As Long
        Dim pEbMode As Long
        Dim pCallback As Long
        Dim pKill As Long
    #End If
    
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
    
    pCallback = GetAddressOf(AddressOf RiffTimerCallback)
    pKill = GetProcAddress(GetModuleHandleA("user32.dll"), "KillTimer")
    
    #If Win64 Then
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
    #Else
        hexStr = "5589E5B80000000085C07421FFD083F801741A83F80274158B4510508B450850B8" & _
                 "0000000085C0741FFFD0EB1B8B4514508B4510508B450C508B450850B800000000" & _
                 "85C07402FFD05DC21000"
        ReDim opcodes(0 To (Len(hexStr) \ 2) - 1)
        For i = 0 To UBound(opcodes)
            opcodes(i) = CByte("&H" & Mid$(hexStr, (i * 2) + 1, 2))
        Next i
        RtlMoveMemory VarPtr(opcodes(4)), VarPtr(pEbMode), 4
        RtlMoveMemory VarPtr(opcodes(33)), VarPtr(pKill), 4
        RtlMoveMemory VarPtr(opcodes(62)), VarPtr(pCallback), 4
    #End If
    
    RtlMoveMemory ByVal rCtx.ThunkTimerCB, VarPtr(opcodes(0)), UBound(opcodes) + 1
    InitThunks = True
End Function

'/**
' * @function FreeThunks
' * @brief Cleans up the runtime-compiled machine code memory.
' */
Private Sub FreeThunks()
    If rCtx.ThunkTimerCB <> 0 Then
        VirtualFree rCtx.ThunkTimerCB, 0, MEM_RELEASE
    End If
    rCtx.ThunkTimerCB = 0
End Sub


'/**
' * @function RiffMixFormatIsFloat32
' * @brief Detects whether the current render format stores samples as 32-bit IEEE float.
' * @return {Boolean} True when the current WASAPI mix format is float32.
' */
Private Function RiffMixFormatIsFloat32() As Boolean
    If rCtx.MixFormatPtr = 0 Then
        Exit Function
    End If

    Dim formatTag As Integer
    Dim bitsPerSample As Integer
    Dim subFormatData1 As Long

    RtlMoveMemory VarPtr(formatTag), ByVal rCtx.MixFormatPtr, 2
    RtlMoveMemory VarPtr(bitsPerSample), ByVal (rCtx.MixFormatPtr + 14), 2

    If bitsPerSample <> 32 Then
        Exit Function
    End If

    If formatTag = 3 Then
        RiffMixFormatIsFloat32 = True
        Exit Function
    End If

    If formatTag = -2 Then
        RtlMoveMemory VarPtr(subFormatData1), ByVal (rCtx.MixFormatPtr + 24), 4
        RiffMixFormatIsFloat32 = (subFormatData1 = 3)
    End If
End Function

'/**
' * @function RiffTryPromoteMixFormatToFloat32
' * @brief Rewrites a WAVEFORMATEX/WAVEFORMATEXTENSIBLE structure to 32-bit float while preserving rate and channel count.
' */
Private Sub RiffTryPromoteMixFormatToFloat32()
    If rCtx.MixFormatPtr = 0 Then
        Exit Sub
    End If

    Dim nChannels As Integer
    Dim sampleRate As Long
    Dim blockAlign As Integer
    Dim avgBytes As Long
    Dim bits As Integer
    Dim cbSize As Integer
    Dim validBits As Integer
    Dim channelMask As Long
    Dim formatTag As Integer

    RtlMoveMemory VarPtr(nChannels), ByVal (rCtx.MixFormatPtr + 2), 2
    RtlMoveMemory VarPtr(sampleRate), ByVal (rCtx.MixFormatPtr + 4), 4
    RtlMoveMemory VarPtr(cbSize), ByVal (rCtx.MixFormatPtr + 16), 2

    If nChannels <= 0 Then
        nChannels = 2
    End If
    If sampleRate <= 0 Then
        sampleRate = 44100
    End If

    bits = 32
    blockAlign = CInt(nChannels * 4)
    avgBytes = sampleRate * CLng(blockAlign)

    If cbSize >= 22 Then
        RtlMoveMemory VarPtr(channelMask), ByVal (rCtx.MixFormatPtr + 20), 4
        formatTag = -2
        validBits = 32
        If channelMask = 0 Then
            If nChannels = 1 Then
                channelMask = 4
            ElseIf nChannels = 2 Then
                channelMask = 3
            End If
        End If

        RtlMoveMemory ByVal rCtx.MixFormatPtr, VarPtr(formatTag), 2
        RtlMoveMemory ByVal (rCtx.MixFormatPtr + 2), VarPtr(nChannels), 2
        RtlMoveMemory ByVal (rCtx.MixFormatPtr + 4), VarPtr(sampleRate), 4
        RtlMoveMemory ByVal (rCtx.MixFormatPtr + 8), VarPtr(avgBytes), 4
        RtlMoveMemory ByVal (rCtx.MixFormatPtr + 12), VarPtr(blockAlign), 2
        RtlMoveMemory ByVal (rCtx.MixFormatPtr + 14), VarPtr(bits), 2
        RtlMoveMemory ByVal (rCtx.MixFormatPtr + 16), VarPtr(cbSize), 2
        RtlMoveMemory ByVal (rCtx.MixFormatPtr + 18), VarPtr(validBits), 2
        RtlMoveMemory ByVal (rCtx.MixFormatPtr + 20), VarPtr(channelMask), 4

        Dim sf1 As Long
        Dim sf2 As Integer
        Dim sf3 As Integer
        Dim sf4(0 To 7) As Byte

        sf1 = 3
        sf2 = 0
        sf3 = 16
        sf4(0) = 128
        sf4(1) = 0
        sf4(2) = 0
        sf4(3) = 170
        sf4(4) = 0
        sf4(5) = 56
        sf4(6) = 155
        sf4(7) = 113

        RtlMoveMemory ByVal (rCtx.MixFormatPtr + 24), VarPtr(sf1), 4
        RtlMoveMemory ByVal (rCtx.MixFormatPtr + 28), VarPtr(sf2), 2
        RtlMoveMemory ByVal (rCtx.MixFormatPtr + 30), VarPtr(sf3), 2
        RtlMoveMemory ByVal (rCtx.MixFormatPtr + 32), VarPtr(sf4(0)), 8
    Else
        formatTag = 3
        cbSize = 0
        RtlMoveMemory ByVal rCtx.MixFormatPtr, VarPtr(formatTag), 2
        RtlMoveMemory ByVal (rCtx.MixFormatPtr + 2), VarPtr(nChannels), 2
        RtlMoveMemory ByVal (rCtx.MixFormatPtr + 4), VarPtr(sampleRate), 4
        RtlMoveMemory ByVal (rCtx.MixFormatPtr + 8), VarPtr(avgBytes), 4
        RtlMoveMemory ByVal (rCtx.MixFormatPtr + 12), VarPtr(blockAlign), 2
        RtlMoveMemory ByVal (rCtx.MixFormatPtr + 14), VarPtr(bits), 2
        RtlMoveMemory ByVal (rCtx.MixFormatPtr + 16), VarPtr(cbSize), 2
    End If
End Sub

'/**
' * @function VTableProc
' * @brief Reads a COM method pointer directly from an interface v-table.
' * @param pUnk COM interface pointer.
' * @param vTableIndex Zero-based v-table slot.
' * @return {Long/LongPtr} Native method pointer.
' */
#If VBA7 Then
Private Function VTableProc(ByVal pUnk As LongPtr, ByVal vTableIndex As Long) As LongPtr
    Dim pVtbl As LongPtr
    RtlMoveMemory VarPtr(pVtbl), ByVal pUnk, LenB(pVtbl)
    RtlMoveMemory VarPtr(VTableProc), ByVal (pVtbl + (vTableIndex * LenB(pVtbl))), LenB(pVtbl)
End Function
#Else
Private Function VTableProc(ByVal pUnk As Long, ByVal vTableIndex As Long) As Long
    Dim pVtbl As Long
    RtlMoveMemory VarPtr(pVtbl), ByVal pUnk, 4
    RtlMoveMemory VarPtr(VTableProc), ByVal (pVtbl + (vTableIndex * 4)), 4
End Function
#End If

'/**
' * @function FastVCall0
' * @brief Invokes a COM v-table method without Variant marshaling when the hot path signature is compatible.
' */
#If VBA7 Then
Private Function FastVCall0(ByVal pUnk As LongPtr, ByVal vTableIndex As Long) As Long
#Else
Private Function FastVCall0(ByVal pUnk As Long, ByVal vTableIndex As Long) As Long
#End If
    FastVCall0 = vCall(pUnk, vTableIndex)
End Function

'/**
' * @function FastVCall1
' * @brief Invokes a COM v-table method with one explicit argument on the optimized x64 path.
' */
#If VBA7 Then
Private Function FastVCall1(ByVal pUnk As LongPtr, ByVal vTableIndex As Long, ByVal arg0 As LongPtr) As Long
#Else
Private Function FastVCall1(ByVal pUnk As Long, ByVal vTableIndex As Long, ByVal arg0 As Long) As Long
#End If
    FastVCall1 = vCall(pUnk, vTableIndex, arg0)
End Function

'/**
' * @function FastVCall3
' * @brief Invokes a COM v-table method with three explicit arguments on the optimized x64 path.
' */
#If VBA7 Then
Private Function FastVCall3(ByVal pUnk As LongPtr, ByVal vTableIndex As Long, ByVal arg0 As LongPtr, ByVal arg1 As LongPtr, ByVal arg2 As LongPtr) As Long
#Else
Private Function FastVCall3(ByVal pUnk As Long, ByVal vTableIndex As Long, ByVal arg0 As Long, ByVal arg1 As Long, ByVal arg2 As Long) As Long
#End If
    FastVCall3 = vCall(pUnk, vTableIndex, arg0, arg1, arg2)
End Function

'/**
' * @function InitWASAPI
' * @brief Initializes the Windows Audio Session API to connect with the default sound hardware.
' * @return {Boolean} True if connected to audio driver.
' */
Private Function InitWASAPI() As Boolean
    Dim clsidEnum As GUID
    Dim iidEnum As GUID
    Dim iidAudio As GUID
    Dim iidRender As GUID
    Dim hr As Long
    
    #If Win64 Then
        Dim pNullPtr As LongLong
        Dim hnsDur As LongLong
        Dim hnsPer As LongLong
        pNullPtr = CLngLng(0)
        hnsDur = CLngLng(1500000)
        hnsPer = CLngLng(0)
    #Else
        #If VBA7 Then
            Dim pNullPtr As LongPtr
        #Else
            Dim pNullPtr As Long
        #End If
        Dim hnsDur As Currency
        Dim hnsPer As Currency
        pNullPtr = CLng(0)
        hnsDur = CCur(150)
        hnsPer = CCur(0)
    #End If
    
    IIDFromString StrPtr("{BCDE0395-E52F-467C-8E3D-C4579291692E}"), clsidEnum
    IIDFromString StrPtr("{A95664D2-9614-4F35-A746-DE8DB63617E6}"), iidEnum
    IIDFromString StrPtr("{1CB9AD4C-DBFA-4c32-B178-C2F568A703B2}"), iidAudio
    IIDFromString StrPtr("{F294ACFC-3146-4483-A7BF-ADDCA7C260E2}"), iidRender
    
    hr = CoCreateInstance(clsidEnum, 0, CLSCTX_ALL, iidEnum, rCtx.DeviceEnumerator)
    If hr <> 0 Or rCtx.DeviceEnumerator = 0 Then
        Exit Function
    End If
    
    hr = vCall(rCtx.DeviceEnumerator, 4, eRender, eConsole, VarPtr(rCtx.Device))
    If hr <> 0 Or rCtx.Device = 0 Then
        Exit Function
    End If
    
    hr = vCall(rCtx.Device, 3, VarPtr(iidAudio), CLSCTX_ALL, pNullPtr, VarPtr(rCtx.AudioClient))
    If hr <> 0 Or rCtx.AudioClient = 0 Then
        Exit Function
    End If
    
    hr = vCall(rCtx.AudioClient, 8, VarPtr(rCtx.MixFormatPtr))
    If hr <> 0 Or rCtx.MixFormatPtr = 0 Then
        Exit Function
    End If
    
    RtlMoveMemory VarPtr(rCtx.sampleRate), ByVal (rCtx.MixFormatPtr + 4), 4
    RtlMoveMemory VarPtr(rCtx.AvgBytesPerSec), ByVal (rCtx.MixFormatPtr + 8), 4
    
    hr = vCall(rCtx.AudioClient, 3, AUDCLNT_SHAREMODE_SHARED, &H80000000, hnsDur, hnsPer, rCtx.MixFormatPtr, pNullPtr)
    If hr <> 0 Then
        hr = vCall(rCtx.AudioClient, 3, AUDCLNT_SHAREMODE_SHARED, 0&, hnsDur, hnsPer, rCtx.MixFormatPtr, pNullPtr)
    End If
    
    If hr <> 0 Then
        Exit Function
    End If
    
    hr = vCall(rCtx.AudioClient, 4, VarPtr(rCtx.BufferSize))
    If hr <> 0 Then
        Exit Function
    End If
    
    hr = vCall(rCtx.AudioClient, 14, VarPtr(iidRender), VarPtr(rCtx.RenderClient))
    If hr <> 0 Or rCtx.RenderClient = 0 Then
        Exit Function
    End If
    
    hr = vCall(rCtx.AudioClient, 10)
    If hr <> 0 Then
        Exit Function
    End If
    
    InitWASAPI = True
End Function

'/**
' * @function ReleaseWASAPI
' * @brief Shuts down COM interfaces for the hardware connections.
' */
Private Sub ReleaseWASAPI()
    If rCtx.RenderClient <> 0 Then
        vCall rCtx.RenderClient, 2
        rCtx.RenderClient = 0
    End If
    
    If rCtx.AudioClient <> 0 Then
        vCall rCtx.AudioClient, 11
        vCall rCtx.AudioClient, 2
        rCtx.AudioClient = 0
    End If
    
    If rCtx.Device <> 0 Then
        vCall rCtx.Device, 2
        rCtx.Device = 0
    End If
    
    If rCtx.DeviceEnumerator <> 0 Then
        vCall rCtx.DeviceEnumerator, 2
        rCtx.DeviceEnumerator = 0
    End If
    
    If rCtx.MixFormatPtr <> 0 Then
        CoTaskMemFree rCtx.MixFormatPtr
        rCtx.MixFormatPtr = 0
    End If
End Sub

'/**
' * @function vCall
' * @brief Low-level C++ Virtual Table invoker to execute COM methods directly in VBA.
' * @param pUnk The pointer to the IUnknown COM interface.
' * @param vTableIndex The 0-based index of the method in the v-table.
' * @param args Array of variant parameters mapped directly to the C++ method signature.
' * @return {Long} The HRESULT returned by the COM method.
' */
#If VBA7 Then
Private Function vCall(ByVal pUnk As LongPtr, ByVal vTableIndex As Long, ParamArray args() As Variant) As Long
#Else
Private Function vCall(ByVal pUnk As Long, ByVal vTableIndex As Long, ParamArray args() As Variant) As Long
#End If
    Dim hrInvoke As Long
    Dim i As Long
    Dim argCount As Long
    Dim offset As Long
    Dim vRet As Variant
    
    #If Win64 Then
        offset = vTableIndex * 8
    #Else
        offset = vTableIndex * 4
    #End If
    
    argCount = 0
    On Error Resume Next
    argCount = UBound(args) - LBound(args) + 1
    If Err.Number <> 0 Then
        Err.Clear
        argCount = 0
    End If
    On Error GoTo 0
    
    If argCount > 0 Then
        #If VBA7 Then
            Dim vArgs() As Variant
            Dim vTypes() As Integer
            Dim pArgs() As LongPtr
        #Else
            Dim vArgs() As Variant
            Dim vTypes() As Integer
            Dim pArgs() As Long
        #End If
        
        ReDim vArgs(0 To argCount - 1)
        ReDim vTypes(0 To argCount - 1)
        ReDim pArgs(0 To argCount - 1)
        
        For i = 0 To argCount - 1
            vArgs(i) = args(i)
            vTypes(i) = VarType(vArgs(i))
            pArgs(i) = VarPtr(vArgs(i))
        Next i
        
        hrInvoke = DispCallFunc(pUnk, offset, CC_STDCALL, vbLong, argCount, vTypes(0), pArgs(0), vRet)
    Else
        hrInvoke = DispCallFunc(pUnk, offset, CC_STDCALL, vbLong, 0, ByVal 0&, ByVal 0&, vRet)
    End If
    
    If hrInvoke = 0 Then
        vCall = CLng(vRet)
    Else
        vCall = hrInvoke
    End If
End Function


