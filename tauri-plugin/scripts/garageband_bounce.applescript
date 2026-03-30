on run argv
    if (count of argv) is less than 1 then
        return "ERR:Expected at least 1 argument: <project_path>"
    end if

    set projectPath to item 1 of argv
    set targetProjectName to my project_name_from_path(projectPath)

    set readinessTimeoutSeconds to 45
    set dialogTimeoutSeconds to 60
    set pollIntervalSeconds to 0.25
    set requiredStableChecks to 3

    tell application "GarageBand"
        activate
        open POSIX file projectPath
    end tell

    set projectReady to my wait_for_target_project_ready(targetProjectName, readinessTimeoutSeconds, pollIntervalSeconds, requiredStableChecks)
    if projectReady is false then
        return "ERR:Timed out waiting for target project to become active"
    end if

    tell application "System Events"
        if UI elements enabled is false then
            return "ERR:Accessibility permissions are required for UI scripting"
        end if

        tell process "GarageBand"
            set frontmost to true
        end tell
    end tell

    set menuReady to my wait_for_export_menu_ready(readinessTimeoutSeconds, pollIntervalSeconds, requiredStableChecks)
    if menuReady is false then
        return "ERR:Timed out waiting for Share > Export menu item to become enabled"
    end if

    set didClickExport to my click_share_export_menu_item()
    if didClickExport is false then
        return "ERR:Could not click 'Export Song to Disk' menu item under Share"
    end if

    set dialogReady to my wait_for_export_dialog_ready(dialogTimeoutSeconds, pollIntervalSeconds)
    if dialogReady is false then
        return "ERR:Timed out waiting for export dialog to appear"
    end if

    set exportSubmitTime to (current date)
    set didSubmitExport to my submit_export_dialog()
    if didSubmitExport is false then
        return "ERR:Found export dialog, but could not submit it"
    end if

    my print_debug("Export submitted. Waiting for bounce completion...")

    set dismissTimeoutSeconds to 15
    set dialogDismissed to my wait_for_export_dialog_dismissed(dismissTimeoutSeconds, pollIntervalSeconds, requiredStableChecks)
    if dialogDismissed is false then
        return "ERR:Export dialog stayed open after submit; export may not have started"
    end if

    set completionTimeoutSeconds to 1800
    set exportCompleted to my wait_for_export_completion(completionTimeoutSeconds, pollIntervalSeconds, requiredStableChecks)
    if exportCompleted is false then
        return "ERR:Timed out waiting for export completion"
    end if

    set elapsedSeconds to ((current date) - exportSubmitTime) as integer
    my print_debug("Bounce finished. Took " & elapsedSeconds & " seconds to bounce.")

    return "OK:DEFAULT_DIALOG_PATH"
end run

on wait_for_target_project_ready(targetProjectName, timeoutSeconds, pollSeconds, stableChecksRequired)
    set startTime to (current date)
    set stableChecks to 0

    repeat
        if my is_timed_out(startTime, timeoutSeconds) then
            return false
        end if

        set isReadyNow to false
        try
            tell application "GarageBand"
                if (count of documents) > 0 then
                    set activeDocumentName to name of document 1
                    ignoring case
                        if targetProjectName is in activeDocumentName then
                            set isReadyNow to true
                        end if
                    end ignoring
                end if
            end tell
        end try

        if isReadyNow then
            set stableChecks to stableChecks + 1
            if stableChecks >= stableChecksRequired then
                return true
            end if
        else
            set stableChecks to 0
        end if

        delay pollSeconds
    end repeat
end wait_for_target_project_ready

on wait_for_export_menu_ready(timeoutSeconds, pollSeconds, stableChecksRequired)
    set startTime to (current date)
    set stableChecks to 0

    repeat
        if my is_timed_out(startTime, timeoutSeconds) then
            return false
        end if

        if my is_export_menu_item_enabled() then
            set stableChecks to stableChecks + 1
            if stableChecks >= stableChecksRequired then
                return true
            end if
        else
            set stableChecks to 0
        end if

        delay pollSeconds
    end repeat
end wait_for_export_menu_ready

on wait_for_export_dialog_ready(timeoutSeconds, pollSeconds)
    set startTime to (current date)

    repeat
        if my is_timed_out(startTime, timeoutSeconds) then
            return false
        end if

        if my has_export_dialog() then
            return true
        end if

        delay pollSeconds
    end repeat
end wait_for_export_dialog_ready

on wait_for_export_dialog_dismissed(timeoutSeconds, pollSeconds, stableChecksRequired)
    set startTime to (current date)
    set stableChecks to 0

    repeat
        if my is_timed_out(startTime, timeoutSeconds) then
            return false
        end if

        if my has_export_dialog() then
            set stableChecks to 0
        else
            set stableChecks to stableChecks + 1
            if stableChecks >= stableChecksRequired then
                return true
            end if
        end if

        delay pollSeconds
    end repeat
end wait_for_export_dialog_dismissed

on wait_for_export_completion(timeoutSeconds, pollSeconds, stableChecksRequired)
    set startTime to (current date)
    set stableChecks to 0

    repeat
        if my is_timed_out(startTime, timeoutSeconds) then
            return false
        end if

        set exportBusy to my is_export_in_progress()
        set exportMenuEnabled to my is_export_menu_item_enabled()

        if (exportBusy is false) and exportMenuEnabled then
            set stableChecks to stableChecks + 1
            if stableChecks >= stableChecksRequired then
                return true
            end if
        else
            set stableChecks to 0
        end if

        delay pollSeconds
    end repeat
end wait_for_export_completion

on submit_export_dialog()
    tell application "System Events"
        tell process "GarageBand"
            repeat with w in windows
                repeat with buttonName in {"Export", "Save", "OK"}
                    try
                        if exists button (contents of buttonName) of w then
                            click button (contents of buttonName) of w
                            return true
                        end if
                    end try
                end repeat
            end repeat

            -- Fallback when default button does not expose a stable title.
            keystroke return
            return true
        end tell
    end tell
end submit_export_dialog

on is_export_in_progress()
    tell application "System Events"
        tell process "GarageBand"
            repeat with w in windows
                try
                    set wTitle to name of w
                    ignoring case
                        if (wTitle contains "export") or (wTitle contains "save") or (wTitle contains "share") or (wTitle contains "sharing") then
                            return true
                        end if
                    end ignoring
                end try

                try
                    if exists button "Cancel" of w then
                        return true
                    end if
                end try

                try
                    if (count of progress indicators of w) > 0 then
                        return true
                    end if
                end try
            end repeat
        end tell
    end tell

    return false
end is_export_in_progress

on is_export_menu_item_enabled()
    tell application "System Events"
        tell process "GarageBand"
            set shareMenuBarItem to my find_share_menu_bar_item()
            if shareMenuBarItem is missing value then
                return false
            end if

            tell menu 1 of shareMenuBarItem
                repeat with exportName in {"Export Song to Disk…", "Export Song to Disk...", "Export to Disk…", "Export to Disk..."}
                    if exists menu item (contents of exportName) then
                        return enabled of menu item (contents of exportName)
                    end if
                end repeat
            end tell
        end tell
    end tell

    return false
end is_export_menu_item_enabled

on click_share_export_menu_item()
    tell application "System Events"
        tell process "GarageBand"
            set shareMenuBarItem to my find_share_menu_bar_item()
            if shareMenuBarItem is missing value then
                return false
            end if

            tell menu 1 of shareMenuBarItem
                repeat with exportName in {"Export Song to Disk…", "Export Song to Disk...", "Export to Disk…", "Export to Disk..."}
                    if exists menu item (contents of exportName) then
                        if enabled of menu item (contents of exportName) then
                            click menu item (contents of exportName)
                            return true
                        end if
                    end if
                end repeat
            end tell
        end tell
    end tell

    return false
end click_share_export_menu_item

on has_export_dialog()
    tell application "System Events"
        tell process "GarageBand"
            try
                if (count of windows) is 0 then
                    return false
                end if

                repeat with w in windows
                    try
                        if (count of sheets of w) > 0 then
                            return true
                        end if
                    end try

                    try
                        set wTitle to name of w
                        ignoring case
                            if (wTitle contains "export") or (wTitle contains "save") then
                                return true
                            end if
                        end ignoring
                    end try

                    try
                        if exists button "Export" of w then
                            return true
                        end if
                    end try

                    try
                        if exists button "Save" of w then
                            return true
                        end if
                    end try
                end repeat
            end try
        end tell
    end tell

    return false
end has_export_dialog

on find_share_menu_bar_item()
    tell application "System Events"
        tell process "GarageBand"
            repeat with candidateName in {"Share", "Ablage"}
                if exists menu bar item (contents of candidateName) of menu bar 1 then
                    return menu bar item (contents of candidateName) of menu bar 1
                end if
            end repeat
        end tell
    end tell

    return missing value
end find_share_menu_bar_item

on is_timed_out(startTime, timeoutSeconds)
    return ((current date) - startTime) >= timeoutSeconds
end is_timed_out

on project_name_from_path(projectPath)
    set fileName to do shell script "basename " & quoted form of projectPath
    if fileName ends with ".band" then
        return text 1 thru -6 of fileName
    end if

    return fileName
end project_name_from_path

on print_debug(messageText)
    do shell script "printf %s\\n " & quoted form of ("[garageband_bounce] " & messageText) & " 1>&2"
end print_debug
