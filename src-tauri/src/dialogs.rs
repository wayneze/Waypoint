// Tauri Commands: 在 Windows 资源管理器中打开指定目录或选中文件
#[tauri::command]
pub fn open_in_explorer(path: String) -> Result<(), String> {
    let p = std::path::Path::new(&path);
    if p.exists() {
        let _ = std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("启动资源管理器失败: {}", e))?;
        Ok(())
    } else {
        Err(format!("路径不存在: {}", path))
    }
}

// Tauri Commands: 打开 Windows 原生系统文件夹选择器
#[tauri::command]
pub fn pick_folder(description: Option<String>) -> Result<Option<String>, String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let desc = description.unwrap_or_else(|| "选择 Obsidian 笔记库文件夹".into());
        let desc = desc.replace('\'', "''");

        let script = format!(
            r#"
            Add-Type -AssemblyName System.Windows.Forms
            $f = New-Object System.Windows.Forms.FolderBrowserDialog
            $f.Description = '{desc}'
            $f.ShowNewFolderButton = $true
            if ($f.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {{
                [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
                [Console]::Out.Write($f.SelectedPath)
            }}
        "#
        );

        let output = std::process::Command::new("powershell")
            .args(["-NoProfile", "-STA", "-Command", &script])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("打开系统文件夹选择器失败: {}", e))?;

        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if path.is_empty() {
            Ok(None)
        } else {
            Ok(Some(path))
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(None)
    }
}

pub(crate) fn windows_file_dialog(save: bool, title: &str, filter: &str, default_name: &str) -> Result<Option<String>, String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::ffi::{OsStrExt, OsStringExt};
        use windows_sys::Win32::UI::Controls::Dialogs::*;
        let wide = |s: &str| std::ffi::OsStr::new(s).encode_wide().chain(Some(0)).collect::<Vec<_>>();
        let title = wide(title);
        let filter = wide(&format!("{}\0", filter.replace('|', "\0")));
        let mut filename = vec![0u16; 32768];
        let initial = wide(default_name);
        if initial.len() > filename.len() { return Err("文件路径过长".into()); }
        filename[..initial.len()].copy_from_slice(&initial);
        let extension = wide(std::path::Path::new(default_name).extension().and_then(|s|s.to_str()).unwrap_or(""));
        let mut dialog: OPENFILENAMEW = unsafe { std::mem::zeroed() };
        dialog.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
        dialog.lpstrTitle = title.as_ptr();
        dialog.lpstrFilter = filter.as_ptr();
        dialog.nFilterIndex = 1;
        dialog.lpstrFile = filename.as_mut_ptr();
        dialog.nMaxFile = filename.len() as u32;
        dialog.lpstrDefExt = extension.as_ptr();
        dialog.Flags = OFN_EXPLORER | OFN_NOCHANGEDIR | OFN_PATHMUSTEXIST | if save { OFN_OVERWRITEPROMPT } else { OFN_FILEMUSTEXIST };
        let accepted = unsafe { if save { GetSaveFileNameW(&mut dialog) } else { GetOpenFileNameW(&mut dialog) } };
        if accepted == 0 {
            let error = unsafe { CommDlgExtendedError() };
            return if error == 0 { Ok(None) } else { Err(format!("文件选择器失败：0x{error:08x}")) };
        }
        let length = filename.iter().position(|&c| c == 0).unwrap_or(filename.len());
        Ok(Some(std::ffi::OsString::from_wide(&filename[..length]).to_string_lossy().into()))
    }
    #[cfg(not(target_os = "windows"))]
    { let _ = (save, title, filter, default_name); Err("当前平台尚未提供文件选择器".into()) }
}

#[tauri::command]
pub async fn pick_save_file(title: String, filter: String, default_name: String) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || windows_file_dialog(true, &title, &filter, &default_name)).await.map_err(|e|e.to_string())?
}

#[tauri::command]
pub async fn pick_open_file(title: String, filter: String) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || windows_file_dialog(false, &title, &filter, "")).await.map_err(|e|e.to_string())?
}
