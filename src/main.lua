--label:カスタムオブジェクト
--require:2003801
--information:https://github.com/sevenc-nanashi/budoux.obj2

---$track:横幅
---min=1
---max=1000
---step=1
local width = 400

---$track:サイズ
---min=1
---max=1000
---step=0.01
local size = 40

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
local speed = 1

---$font:フォント
local font = "Yu Gothic UI"

---$color:文字色
local color = 0xffffff

---$check:太字
local bold = false

---$check:斜体
local italic = false

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
    if request.type == "text_layout" then
        local width, height = obj.load("textlayout", request.data)
        mod.push_stack(json.encode({ width = width, height = height }))
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

local callback = ffi.cast("void (*)(const char*)", lua_callback)
local callback_address = tostring(ffi.cast("intptr_t", callback))
local layout_success, layout_err = pcall(function()
    print(
        {
            lua_callback = callback_address,
            width = width,
            align = align,
            justify = justify,
            text = text,
            size = size,
            letter_spacing = letter_spacing,
            line_spacing = line_spacing,
            show_speed = speed,
            font = font,
            color = color,
            bold = bold,
            italic = italic,
        }

    )
    mod.layout(
        {
            lua_callback = callback_address,
            width = width,
            align = align,
            justify = justify,
            text = text,
            size = size,
            letter_spacing = letter_spacing,
            line_spacing = line_spacing,
            show_speed = speed,
            font = font,
            color = color,
            bold = bold,
            italic = italic,
        }
    )
end)

callback:free()
if not layout_success then
    print("@error", "Failed to initialize text layout:", layout_err)
end