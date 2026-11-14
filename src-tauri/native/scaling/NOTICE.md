Waypoint 的画面缩放组件使用 Magpie 0.12.1 的原生核心，通过独立进程与 Waypoint 通信。

上游：https://github.com/Blinue/Magpie
固定提交：664e0f4c8a9aca4e6efb0e37f52be6dc62414f7b
许可证：GNU GPL version 3 or later，全文见 LICENSE。

vendor 保留上游核心、共享代码、构建属性和本产品实际使用的着色器。host.cpp 是 Waypoint 的宿主接线，按同一 GPL 许可证提供。未修改上游核心。Anime4K 和 FSR 的上游归属保留在对应着色器源码中。

build.ps1 在本项目 target/scaling 内恢复固定版本依赖、编译宿主并生成嵌入程序的 payload.tar.gz。无需安装 Magpie，也不注册系统服务。首次源构建需要网络、Python 3、VS 2022 C++ 工具和 Windows SDK 10.0.26100.0；后续构建使用本地缓存。
