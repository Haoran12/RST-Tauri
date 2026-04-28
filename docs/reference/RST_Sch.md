# RST系统的会话调度模式
分为2类：
- ST模式
- Agent模式

## ST模式
即SillyTavern式的调度方式. 每轮对话一次请求-返回工作流程如下:
- 用户输入
- 根据当前会话采用的Preset, 按顺序寻找各条目内容, 构筑Prompt
- 其中, Lore Before, User Description, Chat History,Lore After, Scene 这几条的content需要系统根据会话配置和聊天记录组装;
- 关键的是两种Lore, 依据用户设定的条件激活条目, 并根据条目注入位置属性决定条目在总Prompt中的位置
### 目录结构
数据存储目录即应用本身所在路径, 不占用AppData等系统目录, 方便用户管理和备份. 目录结构如下:
```./data
├── lores/
│   ├── lxx.json
│   ├── lyy.json
├── presets/
│   ├── pxx.json
│   ├── pyy.json
├── chats/
│   ├── cxx.json
│   ├── cyy.json
├── characters/
│   ├── cxx.json
│   ├── cyy.json
├── api_configs/
│   ├── axx.json
│   ├── ayy.json


```

### ST模式-Lore注入
ST模式的Lore注入追求复刻SillyTavern的效果.
[ST_mode注入流程说明](SillyTavernLorebook.md)


## Agent模式
Agent模式有着完全不同的目录结构和工作流程.

### 目录结构：
同样是以应用所在路径为数据存储目录, 不占用AppData等系统目录目录结构如下:
```./data
├── worlds/
│   ├── wxx/                  
│   ├── wyy/

```
### 流程与数据设计
Agent模式的核心是Worlds, 每个World代表一个故事世界，在自己的目录内存储动态更新的世界信息、人物状态和聊天记录.
