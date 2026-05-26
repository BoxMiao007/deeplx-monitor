# 分析页面 UI 优化

## 需求

### 1. 圆环图区域改为统计卡片网格
- 参考 cpa-usage-keeper 的 StatCards 布局（12列网格，前2个 span 6，后面 span 4）
- 每个卡片使用现有 StatCard 样式（accent 渐变背景、顶部彩色边框、hover 上浮效果）
- 包含圆环图作为卡片内容的一部分
- 数据来源：endpoints 的请求数和字符数

### 2. 热力图默认选择"今天"
- `heatmapDays` 初始值从 7 改为 1

### 3. 语言标识添加 SVG 国旗图标
- 在语言统计表格的语言名称前添加对应国旗 SVG
- 使用内联 SVG（不引入外部图标库）
- 覆盖 DeepLX 常见语言：zh, en, ja, ko, de, fr, es, ru, pt, it, nl, pl, auto 等
- 不确定的语言映射需确认

## 技术方案
- 复用现有 StatCard 组件或扩展其样式
- 国旗 SVG 作为独立映射模块
- 不改变后端 API
