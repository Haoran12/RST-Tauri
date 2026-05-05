//! Character card parsing and exporting
//!
//! PNG 角色卡读写规则：
//! 1. 读取 PNG 时读取文本 metadata chunks
//! 2. 优先级：ccv3 → chara（ST 标准为 base64 编码的 UTF-8 JSON）
//! 3. 导出时写入 chara，并尝试写入 ccv3

use base64::{engine::general_purpose::STANDARD, Engine};
use flate2::read::ZlibDecoder;
use png::{BitDepth, ColorType, Decoder, Encoder};
use serde_json::Value;
use std::io::{BufReader, Cursor, Read};

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
        return Err(
            "PNG 文件不包含有效的 ST 角色卡数据（未找到 ccv3 或 chara metadata chunk）".to_string(),
        );
    };

    let json_bytes = decode_card_metadata_text(&json_text)?;
    parse_character_from_json(&json_bytes)
}

/// 从 JSON 数据解析角色卡
///
/// 解析 TavernCard JSON，保留未知字段。
pub fn parse_character_from_json(json_data: &[u8]) -> Result<TavernCardV3, String> {
    let json_str =
        std::str::from_utf8(json_data).map_err(|e| format!("JSON 不是有效的 UTF-8: {}", e))?;

    let value: Value =
        serde_json::from_str(json_str).map_err(|e| format!("JSON 解析失败: {}", e))?;

    // V3 基础校验
    let spec = value
        .get("spec")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "缺少 spec 字段".to_string())?;

    if spec != "chara_card_v3" {
        // 尝试作为旧版卡处理
        return convert_old_card_to_v3(value);
    }

    let spec_version = value
        .get("spec_version")
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
    let version_num: f64 = spec_version
        .parse()
        .map_err(|_| format!("spec_version 不是有效数字: {}", spec_version))?;

    if version_num < 3.0 || version_num >= 4.0 {
        return Err(format!(
            "spec_version 不在有效范围 [3.0, 4.0): {}",
            spec_version
        ));
    }

    // 解析为 TavernCardV3
    let card: TavernCardV3 =
        serde_json::from_value(value).map_err(|e| format!("角色卡数据解析失败: {}", e))?;

    Ok(card)
}

/// 将旧版角色卡转换为 V3
fn convert_old_card_to_v3(value: Value) -> Result<TavernCardV3, String> {
    // 检查是否是 V2 卡
    let spec = value.get("spec").and_then(|v| v.as_str()).unwrap_or("");

    if spec == "chara_card_v2" {
        // V2 转 V3：修改 spec 和 spec_version
        let mut v3_value = value.clone();
        v3_value["spec"] = Value::String("chara_card_v3".to_string());
        v3_value["spec_version"] = Value::String("3.0".to_string());

        let card: TavernCardV3 =
            serde_json::from_value(v3_value).map_err(|e| format!("V2 卡转换失败: {}", e))?;

        return Ok(card);
    }

    // 检查是否是无 spec 的旧版卡（只有 data 字段）
    if value.get("data").is_some() || value.get("name").is_some() {
        // 构造 V3 结构
        let mut v3_value = serde_json::Map::new();
        v3_value.insert(
            "spec".to_string(),
            Value::String("chara_card_v3".to_string()),
        );
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
    let json_value = serde_json::to_value(card).map_err(|e| format!("角色卡序列化失败: {}", e))?;

    let json_str =
        serde_json::to_string(&json_value).map_err(|e| format!("JSON 字符串化失败: {}", e))?;

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
        encoder
            .add_text_chunk(CHUNK_KEYWORD_CHARA.to_string(), base64_text.clone())
            .map_err(|e| format!("添加 chara chunk 失败: {}", e))?;
        encoder
            .add_text_chunk(CHUNK_KEYWORD_CCV3.to_string(), base64_text)
            .map_err(|e| format!("添加 ccv3 chunk 失败: {}", e))?;

        let mut writer = encoder
            .write_header()
            .map_err(|e| format!("PNG encoder 写入失败: {}", e))?;

        writer
            .write_image_data(&image_data)
            .map_err(|e| format!("PNG 图像数据写入失败: {}", e))?;
    }

    Ok(output)
}

/// 导出角色卡为 JSON
///
/// 导出完整 TavernCard JSON，不包含 RST 内部索引字段。
pub fn export_character_to_json(card: &TavernCardV3) -> Result<Vec<u8>, String> {
    let json_value = serde_json::to_value(card).map_err(|e| format!("角色卡序列化失败: {}", e))?;

    let json_str = serde_json::to_string_pretty(&json_value)
        .map_err(|e| format!("JSON 字符串化失败: {}", e))?;

    Ok(json_str.into_bytes())
}

fn decode_card_metadata_text(text: &str) -> Result<Vec<u8>, String> {
    let trimmed = text.trim_start_matches('\u{feff}').trim_start();
    if trimmed.starts_with('{') {
        return Ok(trimmed.as_bytes().to_vec());
    }

    STANDARD
        .decode(text.trim())
        .map_err(|e| format!("角色卡 metadata 既不是 JSON，也不是有效 Base64: {}", e))
}

/// 从 PNG 提取所有文本 metadata chunks。
fn extract_text_chunks(
    png_data: &[u8],
) -> Result<std::collections::HashMap<String, String>, String> {
    let decoder = Decoder::new(BufReader::new(Cursor::new(png_data)));
    let mut reader = decoder
        .read_info()
        .map_err(|e| format!("PNG 解码失败: {}", e))?;

    let mut image_data = vec![0; reader.output_buffer_size()];
    reader
        .next_frame(&mut image_data)
        .map_err(|e| format!("PNG 读取失败: {}", e))?;

    let mut text_chunks = std::collections::HashMap::new();
    let info = reader.info();

    for text_chunk in &info.uncompressed_latin1_text {
        text_chunks.insert(text_chunk.keyword.clone(), text_chunk.text.clone());
    }
    for text_chunk in &info.compressed_latin1_text {
        let text = text_chunk
            .get_text()
            .map_err(|e| format!("PNG zTXt metadata 解压失败: {}", e))?;
        text_chunks.insert(text_chunk.keyword.clone(), text);
    }
    for text_chunk in &info.utf8_text {
        let text = text_chunk
            .get_text()
            .map_err(|e| format!("PNG iTXt metadata 解压失败: {}", e))?;
        text_chunks.insert(text_chunk.keyword.clone(), text);
    }

    for (keyword, text) in scan_png_text_chunks(png_data)? {
        text_chunks.insert(keyword, text);
    }

    Ok(text_chunks)
}

fn scan_png_text_chunks(png_data: &[u8]) -> Result<Vec<(String, String)>, String> {
    const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
    if png_data.len() < PNG_SIGNATURE.len() || &png_data[..8] != PNG_SIGNATURE {
        return Err("PNG 文件签名无效".to_string());
    }

    let mut chunks = Vec::new();
    let mut cursor = 8usize;
    while cursor + 12 <= png_data.len() {
        let length = u32::from_be_bytes([
            png_data[cursor],
            png_data[cursor + 1],
            png_data[cursor + 2],
            png_data[cursor + 3],
        ]) as usize;
        let chunk_type = &png_data[cursor + 4..cursor + 8];
        let data_start = cursor + 8;
        let data_end = data_start
            .checked_add(length)
            .ok_or_else(|| "PNG chunk 长度溢出".to_string())?;
        let next = data_end
            .checked_add(4)
            .ok_or_else(|| "PNG chunk CRC 长度溢出".to_string())?;
        if next > png_data.len() {
            return Err("PNG chunk 数据不完整".to_string());
        }

        let data = &png_data[data_start..data_end];
        match chunk_type {
            b"tEXt" => {
                if let Some((keyword, text)) = parse_text_chunk(data) {
                    chunks.push((keyword, text));
                }
            }
            b"zTXt" => {
                if let Some((keyword, text)) = parse_ztxt_chunk(data)? {
                    chunks.push((keyword, text));
                }
            }
            b"iTXt" => {
                if let Some((keyword, text)) = parse_itxt_chunk(data)? {
                    chunks.push((keyword, text));
                }
            }
            _ => {}
        }

        cursor = next;
        if chunk_type == b"IEND" {
            break;
        }
    }

    Ok(chunks)
}

fn parse_text_chunk(data: &[u8]) -> Option<(String, String)> {
    let separator = data.iter().position(|byte| *byte == 0)?;
    let keyword = decode_latin1(&data[..separator]);
    let text = decode_latin1(&data[separator + 1..]);
    Some((keyword, text))
}

fn parse_ztxt_chunk(data: &[u8]) -> Result<Option<(String, String)>, String> {
    let separator = match data.iter().position(|byte| *byte == 0) {
        Some(separator) => separator,
        None => return Ok(None),
    };
    let keyword = decode_latin1(&data[..separator]);
    if !is_character_metadata_keyword(&keyword) {
        return Ok(None);
    }
    if data.len() <= separator + 1 {
        return Ok(None);
    }
    if data[separator + 1] != 0 {
        return Err("PNG zTXt metadata 使用了不支持的压缩方法".to_string());
    }

    let text = inflate_text(&data[separator + 2..], false)?;
    Ok(Some((keyword, text)))
}

fn parse_itxt_chunk(data: &[u8]) -> Result<Option<(String, String)>, String> {
    let keyword_end = match data.iter().position(|byte| *byte == 0) {
        Some(separator) => separator,
        None => return Ok(None),
    };
    if data.len() <= keyword_end + 2 {
        return Ok(None);
    }

    let keyword = decode_latin1(&data[..keyword_end]);
    if !is_character_metadata_keyword(&keyword) {
        return Ok(None);
    }

    let compressed = match data[keyword_end + 1] {
        0 => false,
        1 => true,
        _ => return Err("PNG iTXt metadata 压缩标记无效".to_string()),
    };
    if compressed && data[keyword_end + 2] != 0 {
        return Err("PNG iTXt metadata 使用了不支持的压缩方法".to_string());
    }

    let mut pos = keyword_end + 3;
    let language_end = match find_nul(data, pos) {
        Some(separator) => separator,
        None => return Ok(None),
    };
    pos = language_end + 1;
    let translated_end = match find_nul(data, pos) {
        Some(separator) => separator,
        None => return Ok(None),
    };
    pos = translated_end + 1;

    let text = if compressed {
        inflate_text(&data[pos..], true)?
    } else {
        std::str::from_utf8(&data[pos..])
            .map(|text| text.to_string())
            .map_err(|e| format!("PNG iTXt metadata 不是有效 UTF-8: {}", e))?
    };
    Ok(Some((keyword, text)))
}

fn is_character_metadata_keyword(keyword: &str) -> bool {
    keyword == CHUNK_KEYWORD_CCV3 || keyword == CHUNK_KEYWORD_CHARA
}

fn find_nul(data: &[u8], start: usize) -> Option<usize> {
    data.get(start..)?
        .iter()
        .position(|byte| *byte == 0)
        .map(|offset| start + offset)
}

fn decode_latin1(data: &[u8]) -> String {
    data.iter().map(|byte| *byte as char).collect()
}

fn inflate_text(data: &[u8], utf8: bool) -> Result<String, String> {
    let mut decoder = ZlibDecoder::new(data);
    let mut output = Vec::new();
    decoder
        .read_to_end(&mut output)
        .map_err(|e| format!("PNG metadata 解压失败: {}", e))?;

    if utf8 {
        String::from_utf8(output).map_err(|e| format!("PNG iTXt metadata 不是有效 UTF-8: {}", e))
    } else {
        Ok(decode_latin1(&output))
    }
}

/// 读取 PNG 图像数据
fn read_png_image_data(png_data: &[u8]) -> Result<(png::Info<'_>, Vec<u8>), String> {
    let decoder = Decoder::new(BufReader::new(Cursor::new(png_data)));
    let mut reader = decoder
        .read_info()
        .map_err(|e| format!("PNG 解码失败: {}", e))?;

    let info = reader.info().clone();
    let mut image_data = vec![0; reader.output_buffer_size()];

    // 读取图像数据
    reader
        .next_frame(&mut image_data)
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

        let mut writer = encoder
            .write_header()
            .map_err(|e| format!("PNG encoder 写入失败: {}", e))?;

        writer
            .write_image_data(&image_data)
            .map_err(|e| format!("PNG 图像数据写入失败: {}", e))?;
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use png::text_metadata::{ITXtChunk, TEXtChunk};

    fn sample_card_json(name: &str) -> String {
        serde_json::json!({
            "spec": "chara_card_v3",
            "spec_version": "3.0",
            "data": {
                "name": name,
                "description": "desc",
                "personality": "kind",
                "scenario": "",
                "first_mes": "hello",
                "mes_example": "",
                "extensions": {}
            }
        })
        .to_string()
    }

    fn png_with_header_text(kind: &str, keyword: &str, text: &str) -> Vec<u8> {
        let mut output = Vec::new();
        {
            let mut encoder = Encoder::new(&mut output, 1, 1);
            encoder.set_color(ColorType::Rgba);
            encoder.set_depth(BitDepth::Eight);
            match kind {
                "text" => encoder
                    .add_text_chunk(keyword.to_string(), text.to_string())
                    .unwrap(),
                "ztxt" => encoder
                    .add_ztxt_chunk(keyword.to_string(), text.to_string())
                    .unwrap(),
                "itxt" => encoder
                    .add_itxt_chunk(keyword.to_string(), text.to_string())
                    .unwrap(),
                _ => unreachable!(),
            }
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(&[0, 0, 0, 255]).unwrap();
        }
        output
    }

    fn png_with_trailing_text(keyword: &str, text: &str) -> Vec<u8> {
        let mut output = Vec::new();
        {
            let mut encoder = Encoder::new(&mut output, 1, 1);
            encoder.set_color(ColorType::Rgba);
            encoder.set_depth(BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(&[0, 0, 0, 255]).unwrap();
            writer
                .write_text_chunk(&TEXtChunk::new(keyword.to_string(), text.to_string()))
                .unwrap();
        }
        output
    }

    #[test]
    fn parse_png_reads_header_text_chara_base64() {
        let json = sample_card_json("Header tEXt");
        let png = png_with_header_text("text", CHUNK_KEYWORD_CHARA, &STANDARD.encode(json));

        let card = parse_character_from_png(&png).unwrap();

        assert_eq!(card.data.name, "Header tEXt");
    }

    #[test]
    fn parse_png_reads_compressed_text_chunks() {
        let json = sample_card_json("Compressed zTXt");
        let png = png_with_header_text("ztxt", CHUNK_KEYWORD_CHARA, &STANDARD.encode(json));

        let card = parse_character_from_png(&png).unwrap();

        assert_eq!(card.data.name, "Compressed zTXt");
    }

    #[test]
    fn parse_png_reads_utf8_text_direct_json() {
        let json = sample_card_json("Direct iTXt");
        let png = png_with_header_text("itxt", CHUNK_KEYWORD_CCV3, &json);

        let card = parse_character_from_png(&png).unwrap();

        assert_eq!(card.data.name, "Direct iTXt");
    }

    #[test]
    fn parse_png_reads_trailing_text_chunks_after_image_data() {
        let json = sample_card_json("Trailing tEXt");
        let png = png_with_trailing_text(CHUNK_KEYWORD_CHARA, &STANDARD.encode(json));

        let card = parse_character_from_png(&png).unwrap();

        assert_eq!(card.data.name, "Trailing tEXt");
    }

    #[test]
    fn parse_png_prefers_ccv3_over_chara() {
        let chara_json = sample_card_json("chara");
        let ccv3_json = sample_card_json("ccv3");
        let mut output = Vec::new();
        {
            let mut encoder = Encoder::new(&mut output, 1, 1);
            encoder.set_color(ColorType::Rgba);
            encoder.set_depth(BitDepth::Eight);
            encoder
                .add_text_chunk(CHUNK_KEYWORD_CHARA.to_string(), STANDARD.encode(chara_json))
                .unwrap();
            encoder
                .add_text_chunk(CHUNK_KEYWORD_CCV3.to_string(), STANDARD.encode(ccv3_json))
                .unwrap();
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(&[0, 0, 0, 255]).unwrap();
        }

        let card = parse_character_from_png(&output).unwrap();

        assert_eq!(card.data.name, "ccv3");
    }

    #[test]
    fn decode_card_metadata_text_supports_compressed_itxt_payload() {
        let mut chunk = ITXtChunk::new(CHUNK_KEYWORD_CCV3, sample_card_json("Compressed iTXt"));
        chunk.compressed = true;
        let mut output = Vec::new();
        {
            let mut encoder = Encoder::new(&mut output, 1, 1);
            encoder.set_color(ColorType::Rgba);
            encoder.set_depth(BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(&[0, 0, 0, 255]).unwrap();
            writer.write_text_chunk(&chunk).unwrap();
        }

        let card = parse_character_from_png(&output).unwrap();

        assert_eq!(card.data.name, "Compressed iTXt");
    }
}
