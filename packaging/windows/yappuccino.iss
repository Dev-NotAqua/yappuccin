; Yappuccino's Windows installer. scripts/package-windows.py compiles it with
; Inno Setup 7 under Wine, from the same files as the portable ZIP.
;
; It installs for the person running it, without administrator rights, and can
; start Yappuccino in the tray when they sign in. That is the Run key value
; Settings > Advanced > "Start when you sign in" writes (src/autostart.rs), so
; the installer and the setting turn the same thing on and off.

#ifndef AppVersion
  #error Compile through scripts/package-windows.py, which defines AppVersion, VersionNumber and SourceDir
#endif
; The full preview defines Edition and FileSuffix; the Discord release neither.
; Both share the AppId, so installing one replaces the other.
#ifdef Edition
  #define DisplayName "Yappuccino (" + Edition + ")"
#else
  #define DisplayName "Yappuccino"
  #define FileSuffix ""
#endif

[Setup]
; Windows knows an installed Yappuccino by this; never change it.
AppId={{19FF8D53-8D7E-4E66-91BB-E2F5BE307E9A}
AppName=Yappuccino
AppVersion={#AppVersion}
AppVerName={#DisplayName} {#AppVersion}
AppPublisher=Yappuccino
AppPublisherURL=https://yapfast.com
AppSupportURL=https://github.com/Dev-NotAqua/yapfast/issues
AppUpdatesURL=https://github.com/Dev-NotAqua/yapfast/releases
VersionInfoVersion={#VersionNumber}
; Per user: %LOCALAPPDATA%\Programs\Yappuccino, with no UAC prompt.
PrivilegesRequired=lowest
DefaultDirName={autopf}\Yappuccino
DisableProgramGroupPage=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
OutputBaseFilename=Yappuccino-{#AppVersion}{#FileSuffix}-windows-x64-setup
SetupIconFile=yappuccino.ico
UninstallDisplayIcon={app}\Yappuccino.exe
UninstallDisplayName={#DisplayName}
WizardStyle=modern
Compression=lzma2/ultra64
SolidCompression=yes
; Yappuccino keeps running in the tray with its window closed, so an update or
; uninstall closes it rather than asking.
CloseApplications=force
RestartApplications=no
#ifdef LicenseFile
LicenseFile={#LicenseFile}
#endif

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[CustomMessages]
english.StartOnSignIn=Start Yappuccino automatically when Windows starts
english.StartOnSignInGroup=Startup:

[Tasks]
Name: "startonsignin"; Description: "{cm:StartOnSignIn}"; GroupDescription: "{cm:StartOnSignInGroup}"
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
; README.txt explains the ZIP; the installer needs no instructions.
Source: "{#SourceDir}\*"; Excludes: "README.txt"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\Yappuccino"; Filename: "{app}\Yappuccino.exe"
Name: "{autodesktop}\Yappuccino"; Filename: "{app}\Yappuccino.exe"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "Yappuccino"; ValueData: """{app}\Yappuccino.exe"" --background"; Tasks: startonsignin
; Chosen here, it starts even if Task Manager had turned it off before.
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run"; ValueType: none; ValueName: "Yappuccino"; Flags: deletevalue dontcreatekey; Tasks: startonsignin
; Yappuccino's own links open here; claiming Discord's stays the person's choice.
Root: HKCU; Subkey: "Software\Classes\yappuccino"; ValueType: string; ValueName: ""; ValueData: "URL:Yappuccino Protocol"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\yappuccino"; ValueType: string; ValueName: "URL Protocol"; ValueData: ""; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\yappuccino\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: """{app}\Yappuccino.exe"",0"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\yappuccino\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\Yappuccino.exe"" --open-link ""%1"""; Flags: uninsdeletekey
; Settings keeps the app discord:// links opened before claiming them here;
; uninstalling gives them back (below) and drops what was kept.
Root: HKCU; Subkey: "Software\Yappuccino"; Flags: uninsdeletekeyifempty dontcreatekey
Root: HKCU; Subkey: "Software\Yappuccino\DiscordProtocol"; Flags: uninsdeletekey dontcreatekey
; Each start registers Yappuccino's name and icon for its notifications
; (src/notify.rs); uninstalling forgets them.
Root: HKCU; Subkey: "Software\Classes\AppUserModelId\io.github.yappuccino.Yappuccino"; Flags: uninsdeletekey dontcreatekey
; Uninstalling stops starting at sign-in, however it was turned on.
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: none; ValueName: "Yappuccino"; Flags: uninsdeletevalue dontcreatekey
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run"; ValueType: none; ValueName: "Yappuccino"; Flags: uninsdeletevalue dontcreatekey

[Run]
Filename: "{app}\Yappuccino.exe"; Description: "{cm:LaunchProgram,Yappuccino}"; Flags: nowait postinstall skipifsilent

[UninstallRun]
; Before the files go: discord:// links open in the app that had them again,
; if Settings claimed them for this Yappuccino.
Filename: "{app}\Yappuccino.exe"; Parameters: "--release-discord-links"; Flags: runhidden waituntilterminated; RunOnceId: "ReleaseDiscordLinks"
