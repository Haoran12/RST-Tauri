//! Character card parsing and exporting
//!
//! PNG 角色卡读写规则：
//! 1. 读取 PNG 时只读取 tEXt chunks
//! 2. 优先级：ccv3 → chara（都是 base64 编码的 UTF-8 JSON）
//! 3. 导出时写入 chara，并尝试写入 ccv3

use base64::{engine::general_purpose::STANDARD, Engine};
use png::{Decoder, Encoder, ColorType, BitDepth};
use serde_json::Value;
use std::io::{Cursor, BufReader};

use crate::storage::st_resources::TavernCardV3;

/// PNG tEXt chunk keyword
const CHUNK_KEYWORD_CCV3: &str = "ccv3";
const CHUNK_KEYWORD_CHARA: &str = "chara";

/// 从 PNG 文件解析角色卡
///
/// 读取 PNG tEXt chunks，按 ccv3 → chara 优先级取得 base64 JSON。
/// 失败时返回错误信息，不静默创建空角色。
pub fn parse_character_from_png(png_data: &[u8]) -> Result<TavernCardV3, String> {
    // 提取 tEXt chunks
    let text_chunks = extract_text_chunks(png_data)?;

    // 按 ccv3 → chara 优先级查找
    let json_text = if let Some(ccv3_data) = text_chunks.get(CHUNK_KEYWORD_CCV3) {
        ccv3_data.clone()
    } else if let Some(chara_data) = text_chunks.get(CHUNK_KEYWORD_CHARA) {
        chara_data.clone()
    } else {
        return Err("PNG 文件不包含有效的 ST 角色卡数据（未找到 ccv3 或 chara tEXt chunk）".to_string());
    };

    // Base64 解码
    let json_bytes = STANDARD
        .decode(&json_text)
        .map_err(|e| format!("Base64 解码失败: {}", e))?;

    // 解析 JSON
    parse_character_from_json(&json_bytes)
}

/// 从 JSON 数据解析角色卡
///
/// 解析 TavernCard JSON，保留未知字段。
pub fn parse_character_from_json(json_data: &[u8]) -> Result<TavernCardV3, String> {
    let json_str = std::str::from_utf8(json_data)
        .map_err(|e| format!("JSON 不是有效的 UTF-8: {}", e))?;

    let value: Value = serde_json::from_str(json_str)
        .map_err(|e| format!("JSON 解析失败: {}", e))?;

    // V3 基础校验
    let spec = value.get("spec")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "缺少 spec 字段".to_string())?;

    if spec != "chara_card_v3" {
        // 尝试作为旧版卡处理
        return convert_old_card_to_v3(value);
    }

    let spec_version = value.get("spec_version")
        .and_then(|v| {
            // spec_version 可能是字符串或数字
            if v.is_string() {
                v.as_str().map(|s| s.to_string())
            } else if v.is_number() {
                Some(v.to_string())
            } else {
                None
            }
        })
        .ok_or_else(|| "缺少 spec_version 字段".to_string())?;

    // 检查版本范围
    let version_num: f64 = spec_version.parse()
        .map_err(|_| format!("spec_version 不是有效数字: {}", spec_version))?;

    if version_num < 3.0 || version_num >= 4.0 {
        return Err(format!("spec_version 不在有效范围 [3.0, 4.0): {}", spec_version));
    }

    // 解析为 TavernCardV3
    let card: TavernCardV3 = serde_json::from_value(value)
        .map_err(|e| format!("角色卡数据解析失败: {}", e))?;

    Ok(card)
}

/// 将旧版角色卡转换为 V3
fn convert_old_card_to_v3(value: Value) -> Result<TavernCardV3, String> {
    // 检查是否是 V2 卡
    let spec = value.get("spec")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if spec == "chara_card_v2" {
        // V2 转 V3：修改 spec 和 spec_version
        let mut v3_value = value.clone();
        v3_value["spec"] = Value::String("chara_card_v3".to_string());
        v3_value["spec_version"] = Value::String("3.0".to_string());

        let card: TavernCardV3 = serde_json::from_value(v3_value)
            .map_err(|e| format!("V2 卡转换失败: {}", e))?;

        return Ok(card);
    }

    // 检查是否是无 spec 的旧版卡（只有 data 字段）
    if value.get("data").is_some() || value.get("name").is_some() {
        // 构造 V3 结构
        let mut v3_value = serde_json::Map::new();
        v3_value.insert("spec".to_string(), Value::String("chara_card_v3".to_string()));
        v3_value.insert("spec_version".to_string(), Value::String("3.0".to_string()));

        if value.get("data").is_some() {
            v3_value.insert("data".to_string(), value.get("data").unwrap().clone());
        } else {
            // 旧版卡直接把字段放在顶层，移到 data 下
            let mut data = serde_json::Map::new();
            for (key, val) in value.as_object().unwrap_or(&serde_json::Map::new()) {
                data.insert(key.clone(), val.clone());
            }
            v3_value.insert("data".to_string(), Value::Object(data));
        }

        let card: TavernCardV3 = serde_json::from_value(Value::Object(v3_value))
            .map_err(|e| format!("旧版卡转换失败: {}", e))?;

        return Ok(card);
    }

    Err("无法识别的角色卡格式".to_string())
}

/// 导出角色卡为 PNG
///
/// 删除旧 chara/ccv3 metadata，写入当前 TavernCard JSON。
/// PNG 本身就是头像容器。
pub fn export_character_to_png(
    original_png: &[u8],
    card: &TavernCardV3,
) -> Result<Vec<u8>, String> {
    // 序列化角色卡为 JSON
    let json_value = serde_json::to_value(card)
        .map_err(|e| format!("角色卡序列化失败: {}", e))?;

    let json_str = serde_json::to_string(&json_value)
        .map_err(|e| format!("JSON 字符串化失败: {}", e))?;

    // Base64 编码
    let base64_text = STANDARD.encode(json_str.as_bytes());

    // 读取原始 PNG 图像数据
    let (info, image_data) = read_png_image_data(original_png)?;

    // 创建新 PNG
    let mut output = Vec::new();
    {
        let mut encoder = Encoder::new(&mut output, info.width, info.height);
        encoder.set_color(info.color_type);
        encoder.set_depth(info.bit_depth);

        // 添加 tEXt chunks
        encoder.add_text_chunk(CHUNK_KEYWORD_CHARA.to_string(), base64_text.clone())
            .map_err(|e| format!("添加 chara chunk 失败: {}", e))?;
        encoder.add_text_chunk(CHUNK_KEYWORD_CCV3.to_string(), base64_text)
            .map_err(|e| format!("添加 ccv3 chunk 失败: {}", e))?;

        let mut writer = encoder.write_header()
            .map_err(|e| format!("PNG encoder 写入失败: {}", e))?;

        writer.write_image_data(&image_data)
            .map_err(|e| format!("PNG 图像数据写入失败: {}", e))?;
    }

    Ok(output)
}

/// 导出角色卡为 JSON
///
/// 导出完整 TavernCard JSON，不包含 RST 内部索引字段。
pub fn export_character_to_json(card: &TavernCardV3) -> Result<Vec<u8>, String> {
    let json_value = serde_json::to_value(card)
        .map_err(|e| format!("角色卡序列化失败: {}", e))?;

    let json_str = serde_json::to_string_pretty(&json_value)
        .map_err(|e| format!("JSON 字符串化失败: {}", e))?;

    Ok(json_str.into_bytes())
}

/// 从 PNG 提取所有 tEXt chunks
fn extract_text_chunks(png_data: &[u8]) -> Result<std::collections::HashMap<String, String>, String> {
    let decoder = Decoder::new(BufReader::new(Cursor::new(png_data)));
    let reader = decoder.read_info()
        .map_err(|e| format!("PNG 解码失败: {}", e))?;

    // 获取 info 中的 text chunks
    let info = reader.info();

    let mut text_chunks = std::collections::HashMap::new();

    // 遍历所有 text chunks
    for text_chunk in &info.uncompressed_latin1_text {
        text_chunks.insert(text_chunk.keyword.clone(), text_chunk.text.clone());
    }

    Ok(text_chunks)
}

/// 读取 PNG 图像数据
fn read_png_image_data(png_data: &[u8]) -> Result<(png::Info, Vec<u8>), String> {
    let decoder = Decoder::new(BufReader::new(Cursor::new(png_data)));
    let mut reader = decoder.read_info()
        .map_err(|e| format!("PNG 解码失败: {}", e))?;

    let info = reader.info().clone();
    let mut image_data = vec![0; reader.output_buffer_size()];

    // 读取图像数据
    reader.next_frame(&mut image_data)
        .map_err(|e| format!("PNG 读取失败: {}", e))?;

    Ok((info, image_data))
}

/// 创建默认头像 PNG
///
/// 用于导入 JSON 角色卡时生成 PNG 容器。
pub fn create_default_avatar_png(_name: &str) -> Result<Vec<u8>, String> {
    // 创建一个简单的灰色头像
    let width: u32 = 256;
    let height: u32 = 256;
    let mut image_data = Vec::with_capacity((width * height * 4) as usize);

    // 灰色背景
    for _ in 0..width * height {
        image_data.push(128); // R
        image_data.push(128); // G
        image_data.push(128); // B
        image_data.push(255); // A
    }

    let mut output = Vec::new();
    {
        let mut encoder = Encoder::new(&mut output, width, height);
        encoder.set_color(ColorType::Rgba);
        encoder.set_depth(BitDepth::Eight);

        let mut writer = encoder.write_header()
            .map_err(|e| format!("PNG encoder 写入失败: {}", e))?;

        writer.write_image_data(&image_data)
            .map_err(|e| format!("PNG 图像数据写入失败: {}", e))?;
    }

    Ok(output)
}