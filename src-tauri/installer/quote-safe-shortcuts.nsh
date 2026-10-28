; Based on Tauri NSIS utils (MIT). Pass paths via registers so apostrophes remain literal.
!ifmacrodef SetLnkAppUserModelId
  !macroundef SetLnkAppUserModelId
!endif
!macro SetLnkAppUserModelId shortcut
  Push $8
  StrCpy $8 "${shortcut}"
  !insertmacro ComHlpr_CreateInProcInstance ${CLSID_ShellLink} ${IID_IShellLink} r0 ""
  ${If} $0 P<> 0
    ${IUnknown::QueryInterface} $0 '("${IID_IPersistFile}",.r1)'
    ${If} $1 P<> 0
      ${IPersistFile::Load} $1 '(r8, ${STGM_READWRITE})'
      ${IUnknown::QueryInterface} $0 '("${IID_IPropertyStore}",.r2)'
      ${If} $2 P<> 0
        System::Call 'Oleaut32::SysAllocString(w "${BUNDLEID}") i.r3'
        System::Call '*${SYSSTRUCT_PROPERTYKEY}(${PKEY_AppUserModel_ID})p.r4'
        System::Call '*${SYSSTRUCT_PROPVARIANT}(${VT_BSTR},,&i4 $3)p.r5'
        ${IPropertyStore::SetValue} $2 '($4,$5)'

        System::Call 'Oleaut32::SysFreeString($3)'
        System::Free $4
        System::Free $5
        ${IPropertyStore::Commit} $2 ""
        ${IUnknown::Release} $2 ""
        ${IPersistFile::Save} $1 '(r8,1)'
      ${EndIf}
      ${IUnknown::Release} $1 ""
    ${EndIf}
    ${IUnknown::Release} $0 ""
  ${EndIf}
  Pop $8
!macroend

!ifmacrodef SetShortcutTarget
  !macroundef SetShortcutTarget
!endif
!macro SetShortcutTarget shortcut target
  Push $8
  StrCpy $8 "${shortcut}"
  Push $9
  StrCpy $9 "${target}"
  !insertmacro ComHlpr_CreateInProcInstance ${CLSID_ShellLink} ${IID_IShellLink} r0 ""
  ${If} $0 P<> 0
    ${IUnknown::QueryInterface} $0 '("${IID_IPersistFile}",.r1)'
    ${If} $1 P<> 0
      ${IPersistFile::Load} $1 '(r8, ${STGM_READWRITE})'
      ${IShellLink::SetPath} $0 '(wr9)'
      ${IPersistFile::Save} $1 '(r8,1)'
      ${IUnknown::Release} $1 ""
    ${EndIf}
    ${IUnknown::Release} $0 ""
  ${EndIf}
  Pop $9
  Pop $8
!macroend

!ifmacrodef IsShortcutTarget
  !macroundef IsShortcutTarget
!endif
!macro IsShortcutTarget shortcut target
  Push $8
  StrCpy $8 "${shortcut}"
  ; $0: IShellLink
  ; $1: IPersistFile
  ; $2: Target path
  ; $3: Return value

  StrCpy $3 0
  !insertmacro ComHlpr_CreateInProcInstance ${CLSID_ShellLink} ${IID_IShellLink} r0 ""
  ${If} $0 P<> 0
    ${IUnknown::QueryInterface} $0 '("${IID_IPersistFile}", .r1)'
    ${If} $1 P<> 0
      ${IPersistFile::Load} $1 '(r8, ${STGM_READ})'
      System::Alloc MAX_PATH
      Pop $2
      ${IShellLink::GetPath} $0 '(.r2, ${MAX_PATH}, 0, ${SLGP_RAWPATH})'
      ${If} $2 == "${target}"
        StrCpy $3 1
      ${EndIf}
      System::Free $2
      ${IUnknown::Release} $1 ""
    ${EndIf}
    ${IUnknown::Release} $0 ""
  ${EndIf}
  Pop $8
  Push $3
!macroend
