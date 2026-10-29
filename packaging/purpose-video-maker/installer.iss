#ifndef StageDir
  #error StageDir is required
#endif
#ifndef OutDir
  #error OutDir is required
#endif

[Setup]
AppId={{C4BDC487-8A3B-431B-BC60-05D674C38A82}
AppName=Purpose video maker
AppVersion=0.1.4
AppPublisher=FigCraft Jiho
AppPublisherURL=https://github.com/dukong824/figcraft_jiho
DefaultDirName={localappdata}\Programs\Purpose video maker
DefaultGroupName=Purpose video maker
DisableProgramGroupPage=yes
UninstallDisplayIcon={app}\purpose-video-maker.exe
OutputDir={#OutDir}
OutputBaseFilename=Purpose-video-maker-0.1.4-windows-x64-setup
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
CloseApplications=yes

[Files]
Source: "{#StageDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional shortcuts:"; Flags: unchecked

[Icons]
Name: "{group}\Purpose video maker"; Filename: "{app}\purpose-video-maker.exe"
Name: "{autodesktop}\Purpose video maker"; Filename: "{app}\purpose-video-maker.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\purpose-video-maker.exe"; Description: "Launch Purpose video maker"; Flags: nowait postinstall skipifsilent
