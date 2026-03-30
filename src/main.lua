--label:カスタムオブジェクト
--require:2003801
--information:https://github.com/sevenc-nanashi/budoux.obj2

--group:レイアウト

---$track:横幅
---min=1
---max=1000
---step=1
local width = 400

---$check:両端揃え
local justify = false

---$select:揃え
---左揃え[上]=0
---中央揃え[上]=1
---右揃え[上]=2
---強制両端揃え[上]=3
---左揃え[中]=4
---中央揃え[中]=5
---右揃え[中]=6
---強制両端揃え[中]=7
---左揃え[下]=8
---中央揃え[下]=9
---右揃え[下]=10
---強制両端揃え[下]=11
local align = 0

---$track:字間
---min=-500
---max=500
---step=0.01
local letter_spacing = 0

---$track:行間
---min=-500
---max=500
---step=0.01
local line_spacing = 0

---$track:表示速度
---min=0
---max=100
---step=0.01
---zero_label=瞬時
local speed = 0


--group:フォント

---$track:サイズ
---min=1
---max=1000
---step=0.01
local size = 40

---$font:フォント
local font = "Yu Gothic UI"

---$color:文字色
local color = 0xffffff

---$color:影・縁色
local secondary_color = 0x000000

---$select:装飾タイプ
---標準文字=0
---影付き文字=1
---影付き文字（薄）=2
---縁取り文字=3
---縁取り文字（細）=4
---縁取り文字（太）=5
---縁取り文字（角）=6
local decoration = 0

---$check:太字
local bold = false

---$check:斜体
local italic = false

--group:

---$text:テキスト
local text = "Hello, World!"

--group:高度な設定,false

---$check:デバッグモード
local debug = false

---$value:PI
local PI = {}

-- PIからパラメータを取得

local ffi = require("ffi")
local mod = obj.module("budoux")

---$embed
local json = require("json")

local function lua_callback(str)
    -- local width, height = obj.load("textlayout", ffi.string(str))
    -- mod.push_stack(string.format("%d,%d", width, height))
    local request = json.decode(ffi.string(str))
    if debug then
        print("@verbose", "Received callback:", request)
    end
    if request.type == "text_layout" then
        obj.setfont("", 0, request.data.decoration, 0, 0, false, false, request.data.letter_spacing)
        local text_width, text_height = obj.load("textlayout", request.data.text)
        mod.push_stack(json.encode({ width = text_width, height = text_height }))
    else
        print("@warn", "Unknown request type:", request.type)
        mod.push_stack_error("Unknown request type")
    end
end

local function lua_callback_wrapper(str)
    local success, err = pcall(lua_callback, str)
    if not success then
        print("@warn", "Lua callback error:", err)
        mod.push_stack_error(err)
    end
end

local outline_size = 0
if decoration == 3 then
    outline_size = size * 0.15
elseif decoration == 4 then
    outline_size = size * 0.075
elseif decoration == 5 then
    outline_size = size * 0.225
elseif decoration == 6 then
    outline_size = size * 0.075
end

local callback = ffi.cast("void (*)(const char*)", lua_callback_wrapper)
local callback_address = tostring(ffi.cast("intptr_t", callback))
local layout_success, layout_json_or_err, height = pcall(function()
    return mod.layout(
        {
            lua_callback = callback_address,
            width = width,
            align = align % 4,
            justify = justify,
            text = text,
            size = size,
            letter_spacing = letter_spacing,
            line_spacing = line_spacing,
            show_speed = speed,
            font = font,
            color = color,
            secondary_color = secondary_color,
            decoration = decoration,
            outline_size = outline_size,
            bold = bold,
            italic = italic,
            time = obj.time
        }
    )
end)

callback:free()
if not layout_success then
    error("Layout error: " .. tostring(layout_json_or_err))
end
local layout = json.decode(layout_json_or_err)

local vertical_align = math.floor(align / 4)
obj.setoption("drawtarget", "tempbuffer", width, height)
obj.setfont("", 0, decoration, 0, 0, false, false, letter_spacing)
for _, item in ipairs(layout) do
    obj.load("text", item.content)
    obj.draw(item.position[1] - width / 2, item.position[2] - height / 2)
end
obj.setoption("drawtarget", "framebuffer")
obj.load("tempbuffer")
if vertical_align == 0 then
    obj.cy = -height / 2
elseif vertical_align == 2 then
    obj.cy = height / 2
end