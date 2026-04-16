#!/bin/bash

# --- 配置区 ---
PROXY_PORT=7897
DOCKER_CONF_FILE="/etc/systemd/system/docker.service.d/proxy.conf"
WRAPPER_PATH="/usr/local/bin/px"

if [[ $EUID -ne 0 ]]; then
   echo "❌ 请使用 sudo 运行"
   exit 1
fi

enable_proxy() {
    echo "⚙️ 正在配置系统级环境..."

    # 1. 配置 Docker Daemon (用于 pull)
    mkdir -p /etc/systemd/system/docker.service.d
    cat <<EOF > "$DOCKER_CONF_FILE"
[Service]
Environment="HTTP_PROXY=http://127.0.0.1:$PROXY_PORT"
Environment="HTTPS_PROXY=http://127.0.0.1:$PROXY_PORT"
EOF
    systemctl daemon-reload && systemctl restart docker
    echo "✅ Docker Daemon 代理已就绪"

    # 2. 创建 px 包装器 (不修改 zshrc，直接生成全局可执行文件)
    # 这个包装器会自动计算 docker0 IP 并注入环境变量，然后执行你输入的任何指令
    cat <<EOF > "$WRAPPER_PATH"
#!/bin/bash
# 动态获取网桥 IP
HOST_IP=\$(ip addr show docker0 | grep -Po 'inet \K[\d.]+' | head -n1)
if [ -z "\$HOST_IP" ]; then HOST_IP="172.17.0.1"; fi

# 注入环境变量并执行后续命令
HTTP_PROXY="http://\$HOST_IP:$PROXY_PORT" \\
HTTPS_PROXY="http://\$HOST_IP:$PROXY_PORT" \\
http_proxy="http://\$HOST_IP:$PROXY_PORT" \\
https_proxy="http://\$HOST_IP:$PROXY_PORT" \\
"\$@"
EOF
    chmod +x "$WRAPPER_PATH"
    echo "✅ 代理包装器 'px' 已创建"
    echo -e "\n🔥 现在你可以通过在命令前加 'px' 来使用代理，例如："
    echo "   px docker build -t arceos ."
    echo "   px wget http://example.com"
}

disable_proxy() {
    echo "🧹 正在还原系统环境..."
    rm -f "$DOCKER_CONF_FILE"
    systemctl daemon-reload && systemctl restart docker
    rm -f "$WRAPPER_PATH"
    echo "✅ 所有代理设置与 'px' 指令已清除"
}

case "$1" in
    on) enable_proxy ;;
    off) disable_proxy ;;
    *) echo "用法: sudo $0 {on|off}" ;;
esac