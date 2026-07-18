!include "MUI2.nsh"
!include "FileFunc.nsh"
!include "x64.nsh"

; Основные параметры установщика
Name "{{product_name}}"
OutFile "{{output_path}}"
Unicode true
{{#if install_mode_per_machine}}
InstallDir "$PROGRAMFILES\{{product_name}}"
{{else}}
InstallDir "$LOCALAPPDATA\Programs\{{product_name}}"
{{/if}}

; Запрос требуемых прав
{{#if install_mode_per_machine}}
RequestExecutionLevel admin
{{else if install_mode_both}}
RequestExecutionLevel admin
{{else}}
RequestExecutionLevel user
{{/if}}

; Метаданные версии
VIProductVersion "{{version}}.0"
VIAddVersionKey "ProductName" "{{product_name}}"
VIAddVersionKey "FileVersion" "{{version}}"
VIAddVersionKey "ProductVersion" "{{version}}"
VIAddVersionKey "FileDescription" "{{short_description}}"
{{#if publisher}}
VIAddVersionKey "CompanyName" "{{publisher}}"
{{/if}}
{{#if copyright}}
VIAddVersionKey "LegalCopyright" "{{copyright}}"
{{/if}}

; Настройки MUI
!define MUI_ABORTWARNING
{{#if installer_icon}}
!define MUI_ICON "{{installer_icon}}"
{{/if}}
{{#if header_image}}
!define MUI_HEADERIMAGE
!define MUI_HEADERIMAGE_BITMAP "{{header_image}}"
{{/if}}
{{#if sidebar_image}}
!define MUI_WELCOMEFINISHPAGE_BITMAP "{{sidebar_image}}"
{{/if}}

; Страницы установщика
{{#if license}}
!insertmacro MUI_PAGE_LICENSE "{{license}}"
{{/if}}
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

; Страницы удаления
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

; Языки
!insertmacro MUI_LANGUAGE "English"
{{#each additional_languages}}
!insertmacro MUI_LANGUAGE "{{this}}"
{{/each}}

Var SkipWebView2

Function .onInit
    StrCpy $SkipWebView2 "0"
    ${GetParameters} $0
    ClearErrors
    ${GetOptions} "$0" "/SKIP_WEBVIEW2" $1
    IfErrors cheenhub_skip_webview2_done
    StrCpy $SkipWebView2 "1"
cheenhub_skip_webview2_done:
FunctionEnd

; Секция установки
Section "Install"
    SetOutPath $INSTDIR

    ; Запущенный updater может удерживать старый бинарник. Переименование освобождает исходный путь.
    IfFileExists "$INSTDIR\{{main_binary_name}}" 0 cheenhub_install_main_binary
    Delete /REBOOTOK "$INSTDIR\{{main_binary_name}}.previous"
    ClearErrors
    Rename "$INSTDIR\{{main_binary_name}}" "$INSTDIR\{{main_binary_name}}.previous"
    IfErrors cheenhub_replace_main_binary_failed cheenhub_install_main_binary

cheenhub_install_main_binary:
    ClearErrors
    File "{{main_binary_path}}"
    IfErrors cheenhub_install_main_binary_failed cheenhub_cleanup_previous_binary

cheenhub_replace_main_binary_failed:
    SetErrorLevel 1
    Abort "Не удалось освободить исполняемый файл CheenHub для обновления."

cheenhub_install_main_binary_failed:
    Delete "$INSTDIR\{{main_binary_name}}"
    Rename "$INSTDIR\{{main_binary_name}}.previous" "$INSTDIR\{{main_binary_name}}"
    SetErrorLevel 1
    Abort "Не удалось установить новый исполняемый файл CheenHub."

cheenhub_cleanup_previous_binary:
    Delete /REBOOTOK "$INSTDIR\{{main_binary_name}}.previous"
    {{#if installer_icon}}
    File /oname=app.ico "{{installer_icon}}"
    {{/if}}

    ; Установка ресурсов
    {{#each staged_files}}
    SetOutPath "$INSTDIR{{#if this.target_dir}}\{{this.target_dir}}{{/if}}"
    File "{{this.source}}"
    {{/each}}

    SetOutPath $INSTDIR

    ; Создание деинсталлятора
    WriteUninstaller "$INSTDIR\uninstall.exe"

    ; Ярлыки в меню Пуск
    CreateDirectory "$SMPROGRAMS\{{start_menu_folder}}"
    {{#if installer_icon}}
    CreateShortcut "$SMPROGRAMS\{{start_menu_folder}}\{{product_name}}.lnk" "$INSTDIR\{{main_binary_name}}" "" "$INSTDIR\app.ico" 0
    {{else}}
    CreateShortcut "$SMPROGRAMS\{{start_menu_folder}}\{{product_name}}.lnk" "$INSTDIR\{{main_binary_name}}"
    {{/if}}
    CreateShortcut "$SMPROGRAMS\{{start_menu_folder}}\Uninstall {{product_name}}.lnk" "$INSTDIR\uninstall.exe"

    ; Ярлык на рабочем столе
    {{#if installer_icon}}
    CreateShortcut "$DESKTOP\{{product_name}}.lnk" "$INSTDIR\{{main_binary_name}}" "" "$INSTDIR\app.ico" 0
    {{else}}
    CreateShortcut "$DESKTOP\{{product_name}}.lnk" "$INSTDIR\{{main_binary_name}}"
    {{/if}}

    ; Записи для списка установленных приложений
    WriteRegStr SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\{{bundle_id}}" \
        "DisplayName" "{{product_name}}"
    WriteRegStr SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\{{bundle_id}}" \
        "UninstallString" '"$INSTDIR\uninstall.exe"'
    WriteRegStr SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\{{bundle_id}}" \
        "DisplayVersion" "{{version}}"
    {{#if publisher}}
    WriteRegStr SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\{{bundle_id}}" \
        "Publisher" "{{publisher}}"
    {{/if}}
    WriteRegStr SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\{{bundle_id}}" \
        "InstallLocation" "$INSTDIR"

    ; Размер установленного приложения
    ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
    IntFmt $0 "0x%08X" $0
    WriteRegDWORD SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\{{bundle_id}}" \
        "EstimatedSize" "$0"

    {{#if install_webview}}
    StrCmp $SkipWebView2 "1" cheenhub_webview_install_done
    ; Установка WebView2, если она не отключена update-flow.
    {{webview_install_code}}
cheenhub_webview_install_done:
    {{/if}}

SectionEnd

{{#if installer_hooks}}
!include "{{installer_hooks}}"
{{/if}}

; Секция удаления
Section "Uninstall"
    ; Удаление пользовательской регистрации автозапуска CheenHub.
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "CheenHub"
    Delete /REBOOTOK "$INSTDIR\{{main_binary_name}}.previous"
    ; Удаление файлов
    RMDir /r "$INSTDIR"

    ; Удаление ярлыков меню Пуск
    RMDir /r "$SMPROGRAMS\{{start_menu_folder}}"

    ; Удаление ярлыка с рабочего стола
    Delete "$DESKTOP\{{product_name}}.lnk"

    ; Удаление записей приложения
    DeleteRegKey SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\{{bundle_id}}"
SectionEnd
