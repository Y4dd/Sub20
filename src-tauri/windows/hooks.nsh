; Sub20's additions to the Windows installer (Tauri's NSIS hooks).
;
; Registers Sub20 for video files without taking over the default player:
; it joins the "Open with" list, and right-clicking a video offers
; "Compress with Sub20". Several selected videos each start Sub20, and the
; single-instance plugin gathers them into the window that's already open.

!macro SUB20_REGISTER EXT
  WriteRegStr SHCTX "Software\Classes\.${EXT}\OpenWithProgids" "Sub20.Video" ""
  WriteRegStr SHCTX "Software\Classes\SystemFileAssociations\.${EXT}\shell\Sub20" "" "Compress with Sub20"
  WriteRegStr SHCTX "Software\Classes\SystemFileAssociations\.${EXT}\shell\Sub20" "Icon" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\",0"
  WriteRegStr SHCTX "Software\Classes\SystemFileAssociations\.${EXT}\shell\Sub20" "MultiSelectModel" "Player"
  WriteRegStr SHCTX "Software\Classes\SystemFileAssociations\.${EXT}\shell\Sub20\command" "" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\" $\"%1$\""
!macroend

!macro SUB20_UNREGISTER EXT
  DeleteRegValue SHCTX "Software\Classes\.${EXT}\OpenWithProgids" "Sub20.Video"
  DeleteRegKey SHCTX "Software\Classes\SystemFileAssociations\.${EXT}\shell\Sub20"
!macroend

; The video types Sub20 offers itself for. Keep in step with the macOS and
; Linux configs (tauri.macos.conf.json, tauri.linux.conf.json).
!macro SUB20_EACH_TYPE ACTION
  !insertmacro ${ACTION} mp4
  !insertmacro ${ACTION} m4v
  !insertmacro ${ACTION} mkv
  !insertmacro ${ACTION} mov
  !insertmacro ${ACTION} webm
  !insertmacro ${ACTION} avi
  !insertmacro ${ACTION} wmv
  !insertmacro ${ACTION} flv
  !insertmacro ${ACTION} ts
  !insertmacro ${ACTION} m2ts
  !insertmacro ${ACTION} mts
  !insertmacro ${ACTION} mpg
  !insertmacro ${ACTION} mpeg
  !insertmacro ${ACTION} 3gp
!macroend

!macro NSIS_HOOK_POSTINSTALL
  WriteRegStr SHCTX "Software\Classes\Sub20.Video" "" "Video"
  WriteRegStr SHCTX "Software\Classes\Sub20.Video\DefaultIcon" "" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\",0"
  WriteRegStr SHCTX "Software\Classes\Sub20.Video\shell\open" "FriendlyAppName" "Sub20"
  WriteRegStr SHCTX "Software\Classes\Sub20.Video\shell\open\command" "" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\" $\"%1$\""
  !insertmacro SUB20_EACH_TYPE SUB20_REGISTER
  ; SHCNE_ASSOCCHANGED, so Explorer shows the new entries straight away.
  System::Call "shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)"
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ; An update runs the old uninstaller first; the entries stay for the new copy.
  ${If} $UpdateMode <> 1
    !insertmacro SUB20_EACH_TYPE SUB20_UNREGISTER
    DeleteRegKey SHCTX "Software\Classes\Sub20.Video"
    System::Call "shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)"
  ${EndIf}
!macroend
