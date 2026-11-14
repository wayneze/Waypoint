// SPDX-License-Identifier: GPL-3.0-or-later
#include "pch.h"
#include "ScalingOptions.h"
#include "ScalingRuntime.h"
#include "Logger.h"
#include <iostream>
#include <sstream>
#include <mutex>
#include <atomic>

using namespace Magpie;
struct Target { DWORD pid; uint64_t created; };
static std::mutex inputMutex;
static std::vector<Target> targets;
static std::atomic<bool> cancelled{false};
static std::atomic<int> scalingError{-1};

static void Report(const char* phase, const std::string& message = "") {
    std::string escaped;
    for (char c : message) {
        if (c == '"' || c == '\\') escaped += '\\';
        if ((unsigned char)c >= 32) escaped += c;
    }
    std::cout << "{\"phase\":\"" << phase << "\",\"message\":\"" << escaped << "\"}\n" << std::flush;
}
static void Pump(unsigned ms) {
    const auto deadline = GetTickCount64() + ms;
    do {
        MSG msg;
        while (PeekMessage(&msg, nullptr, 0, 0, PM_REMOVE)) {
            if (msg.message == WM_HOTKEY || msg.message == WM_QUIT) cancelled = true;
            TranslateMessage(&msg); DispatchMessage(&msg);
        }
        MsgWaitForMultipleObjectsEx(0, nullptr, 15, QS_ALLINPUT, MWMO_INPUTAVAILABLE);
    } while (GetTickCount64() < deadline);
}
static bool ValidProcess(const Target& target) {
    wil::unique_handle process(OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | SYNCHRONIZE, FALSE, target.pid));
    FILETIME created{}, exited{}, kernel{}, user{};
    return process && WaitForSingleObject(process.get(), 0) == WAIT_TIMEOUT
        && GetProcessTimes(process.get(), &created, &exited, &kernel, &user)
        && ((uint64_t(created.dwHighDateTime) << 32) | created.dwLowDateTime) == target.created;
}
static bool Belongs(HWND hwnd, const std::vector<Target>& current) {
    DWORD pid = 0; GetWindowThreadProcessId(hwnd, &pid);
    return std::any_of(current.begin(), current.end(), [pid](const Target& t) { return t.pid == pid; });
}
static HWND ScalingWindowHandle() {
    HWND hwnd = FindWindow(L"Window_Magpie_967EB565-6F73-4E94-AE53-00CC42592A22", nullptr);
    DWORD pid = 0; GetWindowThreadProcessId(hwnd, &pid);
    return pid == GetCurrentProcessId() ? hwnd : nullptr;
}
static HWND FindGameWindow(const std::vector<Target>& current) {
    struct Search { const std::vector<Target>* current; HWND hwnd = nullptr; LONG area = 0; } search{ &current };
    EnumWindows([](HWND hwnd, LPARAM data) -> BOOL {
        auto& s = *reinterpret_cast<Search*>(data);
        if (!Belongs(hwnd, *s.current) || !IsWindowVisible(hwnd) || IsIconic(hwnd)) return TRUE;
        wchar_t cls[256]{}; GetClassName(hwnd, cls, 256);
        if (std::wstring_view(cls) == L"#32770" || std::wstring_view(cls) == L"ConsoleWindowClass") return TRUE;
        RECT rect{}; GetClientRect(hwnd, &rect);
        DWORD_PTR result = 0;
        if (rect.right >= 200 && rect.bottom >= 120 && rect.right * rect.bottom > s.area
            && SendMessageTimeout(hwnd, WM_NULL, 0, 0, SMTO_ABORTIFHUNG, 100, &result)) {
            s.hwnd = hwnd; s.area = rect.right * rect.bottom;
        }
        return TRUE;
    }, reinterpret_cast<LPARAM>(&search));
    return search.hwnd;
}
static ScalingOptions Options(HWND hwnd, std::string_view preset, std::string_view capture, bool windowed) {
    ScalingOptions o;
    o.flags = windowed ? ScalingFlags::WindowedMode : 0;
    o.initialWindowedScaleFactor = 1.5f;
    o.maxFrameRate = 60.0f;
    o.windowedInitialToolbarState = o.fullscreenInitialToolbarState = ToolbarState::Off;
    o.screenshotsDir = std::filesystem::current_path();
    wchar_t cls[256]{}; GetClassName(hwnd, cls, 256);
    std::wstring name(cls); std::transform(name.begin(), name.end(), name.begin(), towlower);
    // NScripter software surfaces use GDI; modern renderers use WGC. Users can override
    // this one capture decision for a particular installed edition.
    const bool software = capture == "auto" && (name.find(L"onscripter") != name.npos || name.find(L"nscripter") != name.npos);
    o.captureMethod = capture == "gdi" || software ? CaptureMethod::GDI : CaptureMethod::GraphicsCapture;
    auto effect = [&](const char* name, ScalingType type) { o.effects.push_back({.name=name,.scalingType=type}); };
    if (preset == "anime") {
        effect("Anime4K/Anime4K_Upscale_S", ScalingType::Normal);
        effect("Bilinear", ScalingType::Fit);
    } else if (preset == "sharp") {
        effect("FSR/FSR_EASU", ScalingType::Fit);
        effect("FSR/FSR_RCAS", ScalingType::Normal);
    } else effect(preset == "pixel" ? "Nearest" : "Bilinear", ScalingType::Fit);
    for (const auto& item : o.effects)
        std::filesystem::create_directories(std::filesystem::path("cache") / std::filesystem::path(item.name).parent_path());
    o.showToast = [](HWND, std::wstring_view) noexcept {};
    o.showError = [](HWND, ScalingError error) noexcept { scalingError = (int)error; };
    o.save = [](const ScalingOptions&, HWND) noexcept {};
    return o;
}
static const char* ErrorText(int error) {
    switch ((ScalingError)error) {
        case ScalingError::Maximized: return "请先将游戏改为窗口模式，再开启缩放";
        case ScalingError::LowIntegrityLevel: return "游戏权限高于 Waypoint，无法捕获该窗口";
        case ScalingError::CaptureFailed: return "画面捕获失败，可在作品设置中切换 WGC 或 GDI";
        case ScalingError::InvalidSourceWindow: return "游戏窗口不可用，请等待游戏载入后重试";
        default: return "缩放未能启动，请尝试其他捕获方式或基础预设";
    }
}
int wmain(int argc, wchar_t** argv) {
    if (argc != 4) return 2;
    SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX);
    SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    std::filesystem::create_directories(L"logs");
    Logger::Get().Initialize(spdlog::level::info, L"logs\\scaling.log", 500000, 2);
    try {
        winrt::init_apartment(winrt::apartment_type::single_threaded);
        auto resource = winrt::ResourceLoader::GetForViewIndependentUse(L"Magpie/Resources");
        const auto preset = winrt::to_string(argv[1]), capture = winrt::to_string(argv[2]);
        const bool windowed = std::wstring_view(argv[3]) == L"window";
        std::thread([] {
            std::string line;
            while (std::getline(std::cin, line)) {
                if (line == "stop") break;
                if (!line.starts_with("targets ")) continue;
                std::istringstream stream(line.substr(8)); std::vector<Target> next; Target t{};
                while (stream >> t.pid >> t.created) if (ValidProcess(t)) next.push_back(t);
                std::lock_guard lock(inputMutex); targets = std::move(next);
            }
            cancelled = true;
        }).detach();
        RegisterHotKey(nullptr, 1, MOD_CONTROL | MOD_ALT | MOD_NOREPEAT, VK_F10);
        ScalingRuntime runtime;
        HWND source = nullptr; DWORD sourcePid = 0; RECT original{};
        bool requested = false, wasRendered = false, renderedThisStart = false;
        std::string phase; uint64_t focusLost = 0, startAt = 0, readySince = 0, begin = GetTickCount64();
        auto report = [&](const char* value, const std::string& message = "") {
            if (phase != value) { Report(value, message); phase = value; }
        };
        auto stop = [&] {
            if (!requested) return;
            runtime.Stop();
            const auto deadline = GetTickCount64() + 3000;
            do { Pump(30); } while (runtime.State() != ScalingState::Idle && GetTickCount64() < deadline);
            // Window-mode scaling repositions its source. The host owns restoring placement.
            DWORD pid = 0; GetWindowThreadProcessId(source, &pid);
            if (windowed && source && IsWindow(source) && pid == sourcePid)
                SetWindowPos(source, nullptr, original.left, original.top, original.right-original.left,
                    original.bottom-original.top, SWP_NOACTIVATE | SWP_NOZORDER);
            requested = false;
        };
        report("waiting");
        while (!cancelled) {
            Pump(25);
            std::vector<Target> current;
            { std::lock_guard lock(inputMutex); current = targets; }
            std::erase_if(current, [](const Target& t) { return !ValidProcess(t); });
            if (source && (!IsWindow(source) || !Belongs(source, current))) {
                stop(); if (wasRendered && current.empty()) break;
                source = nullptr; readySince = 0;
            }
            HWND candidate = FindGameWindow(current);
            if (!source && candidate) {
                source = candidate; GetWindowThreadProcessId(source, &sourcePid);
                GetWindowRect(source, &original); readySince = GetTickCount64();
            }
            if (!source) {
                if (GetTickCount64() - begin > 90000) { report("error", "未找到可用游戏窗口，请先启动游戏并等待载入"); break; }
                continue;
            }
            const HWND foreground = GetForegroundWindow();
            const bool focused = foreground && (Belongs(foreground, current) || foreground == ScalingWindowHandle());
            if (!focused || IsIconic(source)) {
                if (!focusLost) focusLost = GetTickCount64();
                if (GetTickCount64() - focusLost > 150) { stop(); report("paused"); }
                continue;
            }
            focusLost = 0;
            if (!requested && GetTickCount64() - readySince > 1500) {
                scalingError = -1; runtime.Start(source, Options(source, preset, capture, windowed), false);
                startAt = GetTickCount64(); requested = true; renderedThisStart = false; report("starting");
            }
            if (requested) {
                if (scalingError >= 0) { report("error", ErrorText(scalingError)); break; }
                HWND overlay = ScalingWindowHandle();
                if (overlay && IsWindowVisible(overlay)) { wasRendered = renderedThisStart = true; report("scaling"); }
                if ((runtime.State() == ScalingState::Idle && GetTickCount64() - startAt > 2000)
                    || (!renderedThisStart && GetTickCount64() - startAt > 15000)) {
                    report("error", "缩放已停止，游戏可能仍在载入；载入完成后可重新应用"); break;
                }
            }
        }
        stop(); UnregisterHotKey(nullptr, 1);
        if (phase != "error") Report("idle");
        Logger::Get().Flush();
        return phase == "error" ? 1 : 0;
    } catch (const winrt::hresult_error& error) {
        Report("error", "缩放组件初始化失败：" + winrt::to_string(error.message())); return 1;
    } catch (const std::exception& error) {
        Report("error", error.what()); return 1;
    }
}
