use anyhow::Context;
use serde::{Deserialize, Serialize};
use specta::Type;

use super::{string_to_i64, CategoryRespData, CategorySubRespData, GetComicRespData};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RedirectRespData {
    #[serde(rename = "search_query")]
    pub search_query: String,
    #[serde(deserialize_with = "string_to_i64")]
    pub total: i64,
    #[serde(rename = "redirect_aid", deserialize_with = "positive_redirect_id")]
    pub redirect_aid: i64,
}

fn positive_redirect_id<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    let value = serde_json::Value::deserialize(deserializer)?;
    let id = match value {
        serde_json::Value::Number(number) => number.as_i64(),
        serde_json::Value::String(text) => text.trim().parse::<i64>().ok(),
        _ => None,
    };
    id.filter(|id| *id > 0)
        .ok_or_else(|| D::Error::custom("redirect_aid 必须是有效的正整数或数字字符串"))
}

pub enum SearchResponseData {
    Redirect(RedirectRespData),
    Results(SearchRespData),
}

impl SearchResponseData {
    pub fn parse(data: &str) -> anyhow::Result<Self> {
        let value: serde_json::Value =
            serde_json::from_str(data).context("解析搜索接口数据失败")?;
        // A redirect intentionally has empty content. Never treat a malformed
        // redirect as an ordinary empty search result.
        if value.get("redirect_aid").is_some() {
            Ok(Self::Redirect(
                serde_json::from_value(value).context("解析 ID 搜索重定向失败")?,
            ))
        } else {
            Ok(Self::Results(
                serde_json::from_value(value).context("解析关键词搜索结果失败")?,
            ))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub enum SearchResp {
    SearchRespData(SearchRespData),
    // 用Box包装GetComicRespData，因为GetComicRespData比SearchRespData大得多
    // 如果不用Box包装，即使SearchResp的类型是SearchRespData，也会占用与GetComicRespData一样大的内存
    ComicRespData(Box<GetComicRespData>),
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchRespData {
    #[serde(rename = "search_query")]
    pub search_query: String,
    #[serde(deserialize_with = "string_to_i64")]
    pub total: i64,
    pub content: Vec<ComicInSearchRespData>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ComicInSearchRespData {
    pub id: String,
    pub author: String,
    pub name: String,
    pub image: String,
    pub category: CategoryRespData,
    #[serde(rename = "category_sub")]
    pub category_sub: CategorySubRespData,
    pub liked: bool,
    #[serde(rename = "is_favorite")]
    pub is_favorite: bool,
    #[serde(rename = "update_at")]
    pub update_at: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn numeric_and_string_redirect_ids_open_the_same_comic() {
        for id in [json!(1472763), json!("1472763")] {
            for total in [json!(1), json!("1")] {
                // Structure observed on /search for the user-provided ID.
                let data = json!({
                    "search_query": "1472763", "search_type": "0",
                    "y": 0, "m": 0, "total": total,
                    "redirect_aid": id, "content": [],
                });
                let SearchResponseData::Redirect(redirect) =
                    SearchResponseData::parse(&data.to_string()).unwrap()
                else {
                    panic!("an ID redirect must not become an empty result page")
                };
                assert_eq!(redirect.redirect_aid, 1472763);
                assert_eq!(redirect.total, 1);
            }
        }
    }

    #[test]
    fn keyword_results_and_legitimate_empty_pages_still_parse() {
        let item = json!({
            "id": "900001", "author": "fictional author", "name": "fictional title",
            "image": "", "category": {}, "category_sub": {},
            "liked": false, "is_favorite": false, "update_at": 0,
        });
        for content in [json!([]), json!([item])] {
            let expected = content.as_array().unwrap().len();
            let data = json!({"search_query": "fictional", "total": expected.to_string(), "content": content});
            let SearchResponseData::Results(page) =
                SearchResponseData::parse(&data.to_string()).unwrap()
            else {
                panic!("a keyword result must stay an ordinary result page")
            };
            assert_eq!(page.content.len(), expected);
            assert_eq!(page.total, expected as i64);
        }
    }

    #[test]
    fn invalid_redirects_are_errors_instead_of_empty_search_results() {
        for id in [
            json!(null),
            json!(0),
            json!(-1),
            json!(true),
            json!(""),
            json!("not-an-id"),
            json!(1.5),
            json!(u64::MAX),
            json!("9223372036854775808"),
            json!([]),
            json!({}),
        ] {
            let data =
                json!({"search_query": "1472763", "total": 1, "redirect_aid": id, "content": []});
            let error = SearchResponseData::parse(&data.to_string())
                .err()
                .expect("invalid ID must be rejected");
            assert!(format!("{error:#}").contains("解析 ID 搜索重定向失败"));
        }
    }

    #[test]
    fn malformed_search_payload_is_reported_instead_of_defaulting_to_an_empty_page() {
        for data in [
            "not json",
            "null",
            "[]",
            r#"{"search_query":"fictional","total":1}"#,
        ] {
            assert!(SearchResponseData::parse(data).is_err());
        }
    }
}
