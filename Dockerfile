# 使用轻量级的 Python 3.12 镜像
FROM python:3.12-slim

# 安装 uv (推荐的 Docker 安装方式)
COPY --from=ghcr.io/astral-sh/uv:latest /uv /uvx /bin/

# 设置工作目录
WORKDIR /app

# 复制代理服务脚本
COPY fish_openai_proxy.py /app/

# 暴露容器端口
EXPOSE 8000

# 使用 uv run 启动服务，它会自动解析脚本里的依赖并运行
CMD ["uv", "run", "fish_openai_proxy.py"]
