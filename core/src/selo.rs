use wasm_bindgen::prelude::*;

pub(crate) struct PhoneSummary {
    pub national: String,
    pub formatted: String,
    pub e164: String,
    pub uf: &'static str,
}

struct CepRange {
    uf: &'static str,
    from: i32,
    to: i32,
}

struct BrDoc {
    kind: &'static str,
    label: &'static str,
    valid: bool,
    formatted: Option<String>,
    origin: Option<String>,
    pix_kind: Option<&'static str>,
}

pub fn br_document(
    kind: &str,
    action: &str,
    input: &str,
    uf: &str,
    random: &[u8],
) -> Result<String, JsValue> {
    let kind = normalize_kind(kind);
    let action = action.trim().to_ascii_lowercase();
    let uf = normalize_uf(uf);
    match action.as_str() {
        "generate" => generate_document(kind, uf.as_deref(), random),
        "format" => inspect_document(kind, input, uf.as_deref(), true),
        "detect" => detect_document(input),
        _ => inspect_document(kind, input, uf.as_deref(), false),
    }
}

pub(crate) fn phone_summary(input: &str) -> Option<PhoneSummary> {
    let trimmed = input.trim();
    let digits = only_digits(trimmed);
    if trimmed.starts_with('+') && !digits.starts_with("55") {
        return None;
    }
    if digits.starts_with("00") && !digits.starts_with("0055") {
        return None;
    }
    let national = national_phone_digits(&digits)?;
    let uf = ddd_uf(&national[0..2])?;
    Some(PhoneSummary {
        formatted: mask_phone_national(&national),
        e164: format!("+55{national}"),
        national,
        uf,
    })
}

fn inspect_document(
    kind: &str,
    input: &str,
    uf: Option<&str>,
    require_format: bool,
) -> Result<String, JsValue> {
    if kind == "auto" {
        return detect_document(input);
    }
    let doc = evaluate_document(kind, input, uf).ok_or_else(|| err("unknown Brazilian document kind"))?;
    if require_format && doc.formatted.is_none() {
        return Err(err("document cannot be formatted with the current input"));
    }
    Ok(document_json(&doc, input, "validate", None))
}

fn detect_document(input: &str) -> Result<String, JsValue> {
    let order = [
        "cpf", "cnpj", "pix", "phone-br", "cep", "plate", "cnh", "pis", "renavam",
        "voter-id", "cns", "rg", "ie",
    ];
    for kind in order {
        if let Some(doc) = evaluate_document(kind, input, None) {
            if doc.valid {
                return Ok(document_json(&doc, input, "detect", Some("Detected document type")));
            }
        }
    }
    Ok(format!(
        "{{\"kind\":\"auto\",\"label\":\"Auto detect\",\"action\":\"detect\",\"input\":\"{}\",\"valid\":false,\"summary\":\"No supported Brazilian document matched this value\",\"details\":[\"Supported: CPF, CNPJ, CNH, PIS/PASEP/NIS, RENAVAM, voter ID, CEP, phone, plate, CNS, RG-SP, IE-SP/MG/RS/PR and PIX keys\"]}}",
        json_escape(input.trim())
    ))
}

fn generate_document(kind: &str, uf: Option<&str>, random: &[u8]) -> Result<String, JsValue> {
    let mut rng = BrRng::new(random);
    if kind == "auto" {
        return Err(err("choose a document type to generate"));
    }
    if kind == "person" {
        return generate_person(uf.unwrap_or("SP"), &mut rng);
    }
    let raw = match kind {
        "cpf" => generate_cpf(&mut rng)?,
        "cnpj" => generate_cnpj(&mut rng, false)?,
        "cnh" => generate_cnh(&mut rng)?,
        "pis" => generate_pis(&mut rng)?,
        "renavam" => generate_renavam(&mut rng)?,
        "voter-id" => generate_voter_id(uf, &mut rng)?,
        "cep" => generate_cep(uf, &mut rng)?,
        "phone-br" => generate_phone(uf, &mut rng)?,
        "plate" => generate_plate(&mut rng)?,
        "cns" => generate_cns(&mut rng)?,
        "rg" => generate_rg(&mut rng)?,
        "ie" => generate_ie(uf.unwrap_or("SP"), &mut rng)?,
        "pix" => generate_pix(&mut rng)?,
        _ => return Err(err("unknown Brazilian document kind")),
    };
    let doc = evaluate_document(kind, &raw, uf).ok_or_else(|| err("generated document could not be inspected"))?;
    Ok(document_json(&doc, &raw, "generate", Some("Generated valid document")))
}

fn evaluate_document(kind: &str, input: &str, uf: Option<&str>) -> Option<BrDoc> {
    let kind = normalize_kind(kind);
    let doc = match kind {
        "cpf" => {
            let formatted = format_cpf(input).ok();
            let origin = origin_cpf(input).ok();
            BrDoc { kind: "cpf", label: "CPF", valid: validate_cpf(input), formatted, origin, pix_kind: None }
        }
        "cnpj" => BrDoc {
            kind: "cnpj",
            label: "CNPJ",
            valid: validate_cnpj(input),
            formatted: format_cnpj(input).ok(),
            origin: None,
            pix_kind: None,
        },
        "cnh" => identity_doc("cnh", "CNH", input, 11, validate_cnh(input)),
        "pis" => BrDoc {
            kind: "pis",
            label: "PIS/PASEP/NIS",
            valid: validate_pis(input),
            formatted: format_pis(input).ok(),
            origin: None,
            pix_kind: None,
        },
        "renavam" => BrDoc {
            kind: "renavam",
            label: "RENAVAM",
            valid: validate_renavam(input),
            formatted: format_renavam(input).ok(),
            origin: None,
            pix_kind: None,
        },
        "voter-id" => BrDoc {
            kind: "voter-id",
            label: "Titulo Eleitoral",
            valid: validate_voter_id(input),
            formatted: format_voter_id(input).ok(),
            origin: origin_voter_id(input).ok(),
            pix_kind: None,
        },
        "cep" => BrDoc {
            kind: "cep",
            label: "CEP",
            valid: validate_cep(input),
            formatted: format_cep(input).ok(),
            origin: origin_cep(input).ok(),
            pix_kind: None,
        },
        "phone-br" => {
            let summary = phone_summary(input);
            BrDoc {
                kind: "phone-br",
                label: "Brazil phone",
                valid: summary.is_some(),
                formatted: summary.as_ref().map(|p| p.formatted.clone()),
                origin: summary.map(|p| p.uf.to_string()),
                pix_kind: None,
            }
        }
        "plate" => BrDoc {
            kind: "plate",
            label: "License plate",
            valid: validate_plate(input),
            formatted: format_plate(input).ok(),
            origin: None,
            pix_kind: None,
        },
        "cns" => identity_doc("cns", "CNS", input, 15, validate_cns(input)),
        "rg" => BrDoc {
            kind: "rg",
            label: "RG-SP",
            valid: validate_rg(input, uf.unwrap_or("SP")),
            formatted: format_rg(input).ok(),
            origin: Some("SP".to_string()),
            pix_kind: None,
        },
        "ie" => {
            let d = only_digits(input);
            let found_uf = uf
                .and_then(|state| if validate_ie_for_uf(&d, state) { Some(state.to_string()) } else { None })
                .or_else(|| ["SP", "MG", "RS", "PR"].iter().find(|state| validate_ie_for_uf(&d, state)).map(|state| state.to_string()));
            let formatted = found_uf.as_deref().and_then(|state| format_ie_for_uf(&d, state).ok());
            BrDoc {
                kind: "ie",
                label: "Inscricao Estadual",
                valid: found_uf.is_some(),
                formatted,
                origin: found_uf,
                pix_kind: None,
            }
        }
        "pix" => {
            let pix_kind = detect_pix_kind(input);
            BrDoc {
                kind: "pix",
                label: "PIX key",
                valid: pix_kind.is_some(),
                formatted: pix_kind.map(|_| input.trim().to_string()),
                origin: None,
                pix_kind,
            }
        }
        _ => return None,
    };
    Some(doc)
}

fn identity_doc(kind: &'static str, label: &'static str, input: &str, len: usize, valid: bool) -> BrDoc {
    let d = only_digits(input);
    BrDoc {
        kind,
        label,
        valid,
        formatted: if d.len() == len { Some(d) } else { None },
        origin: None,
        pix_kind: None,
    }
}

fn document_json(doc: &BrDoc, input: &str, action: &str, prefix: Option<&str>) -> String {
    let value = doc.formatted.as_deref().unwrap_or(input.trim());
    let summary = match prefix {
        Some(prefix) => format!("{prefix}: {}", doc_summary_value(doc, value)),
        None if doc.valid => format!("{} is valid: {}", doc.label, doc_summary_value(doc, value)),
        None => format!("{} is invalid", doc.label),
    };
    let mut fields = vec![
        format!("\"kind\":\"{}\"", doc.kind),
        format!("\"label\":\"{}\"", json_escape(doc.label)),
        format!("\"action\":\"{}\"", action),
        format!("\"input\":\"{}\"", json_escape(input.trim())),
        format!("\"valid\":{}", doc.valid),
        format!("\"summary\":\"{}\"", json_escape(&summary)),
    ];
    if let Some(formatted) = &doc.formatted {
        fields.push(format!("\"formatted\":\"{}\"", json_escape(formatted)));
    }
    if let Some(origin) = &doc.origin {
        fields.push(format!("\"origin\":\"{}\"", json_escape(origin)));
    }
    if let Some(pix_kind) = doc.pix_kind {
        fields.push(format!("\"pix_kind\":\"{}\"", pix_kind));
    }
    fields.push(format!("\"details\":[{}]", doc_details(doc).join(",")));
    format!("{{{}}}", fields.join(","))
}

fn doc_summary_value(doc: &BrDoc, value: &str) -> String {
    if let Some(kind) = doc.pix_kind {
        return format!("{value} ({kind})");
    }
    if let Some(origin) = &doc.origin {
        return format!("{value} ({origin})");
    }
    value.to_string()
}

fn doc_details(doc: &BrDoc) -> Vec<String> {
    let mut details = Vec::new();
    details.push(q(if doc.valid { "Check digits and shape passed" } else { "Check digits or shape failed" }));
    if let Some(formatted) = &doc.formatted {
        details.push(q(&format!("Formatted: {formatted}")));
    }
    if let Some(origin) = &doc.origin {
        details.push(q(&format!("Origin: {origin}")));
    }
    if let Some(kind) = doc.pix_kind {
        details.push(q(&format!("PIX kind: {kind}")));
    }
    details
}

fn generate_person(uf: &str, rng: &mut BrRng<'_>) -> Result<String, JsValue> {
    let uf = if UF_CODES.contains(&uf) { uf } else { "SP" };
    let first = FIRST_NAMES[rng.index(FIRST_NAMES.len())?];
    let last = LAST_NAMES[rng.index(LAST_NAMES.len())?];
    let name = format!("{first} {last}");
    let email = format!(
        "{}.{}{}@example.com.br",
        first.to_ascii_lowercase(),
        last.to_ascii_lowercase(),
        rng.index(1000)?
    );
    let cpf = format_cpf(&generate_cpf_for_uf(uf, rng)?).unwrap();
    let cnh = generate_cnh(rng)?;
    let pis = format_pis(&generate_pis(rng)?).unwrap();
    let renavam = generate_renavam(rng)?;
    let voter_id = format_voter_id(&generate_voter_id(Some(uf), rng)?).unwrap();
    let cns = generate_cns(rng)?;
    let cep = format_cep(&generate_cep(Some(uf), rng)?).unwrap();
    let phone_national = generate_phone(Some(uf), rng)?;
    let phone = mask_phone_national(&phone_national);
    let pix_evp = generate_pix(rng)?;
    let plate = generate_plate(rng)?;
    let company_name = format!("{} {}", last, COMPANY_SUFFIXES[rng.index(COMPANY_SUFFIXES.len())?]);
    let cnpj = format_cnpj(&generate_cnpj(rng, false)?).unwrap();
    let rg = if uf == "SP" { format_rg(&generate_rg(rng)?).ok() } else { None };
    let ie = if matches!(uf, "SP" | "MG" | "RS" | "PR") {
        Some(generate_ie(uf, rng)?)
    } else {
        None
    };

    let mut person_fields = vec![
        format!("\"name\":\"{}\"", json_escape(&name)),
        format!("\"email\":\"{}\"", json_escape(&email)),
        format!("\"uf\":\"{}\"", uf),
        format!("\"cpf\":\"{}\"", cpf),
        format!("\"cnh\":\"{}\"", cnh),
        format!("\"pis\":\"{}\"", pis),
        format!("\"renavam\":\"{}\"", renavam),
        format!("\"voter_id\":\"{}\"", voter_id),
        format!("\"cns\":\"{}\"", cns),
        format!("\"cep\":\"{}\"", cep),
        format!("\"phone\":\"{}\"", phone),
        format!("\"pix_keys\":[\"{}\",\"+55{}\",\"{}\",\"{}\"]", cpf, phone_national, json_escape(&email), pix_evp),
        format!("\"vehicle\":{{\"plate\":\"{}\",\"renavam\":\"{}\"}}", plate, generate_renavam(rng)?),
        format!("\"company\":{{\"name\":\"{}\",\"cnpj\":\"{}\"}}", json_escape(&company_name), cnpj),
    ];
    if let Some(rg) = rg {
        person_fields.push(format!("\"rg\":\"{}\"", rg));
    }
    if let Some(ie) = ie {
        person_fields.push(format!("\"ie\":\"{}\"", ie));
    }
    let summary = format!("Generated synthetic Brazilian person for {uf}: {name}");
    Ok(format!(
        "{{\"kind\":\"person\",\"label\":\"Synthetic person\",\"action\":\"generate\",\"valid\":true,\"summary\":\"{}\",\"person\":{{{}}},\"details\":[\"Documents are synthetic fixtures\",\"CPF, voter ID, CEP and phone are UF-consistent\",\"Optional RG appears for SP; IE appears for SP, MG, RS or PR\"]}}",
        json_escape(&summary),
        person_fields.join(",")
    ))
}

fn normalize_kind(kind: &str) -> &str {
    match kind.trim().to_ascii_lowercase().as_str() {
        "cpf" => "cpf",
        "cnpj" => "cnpj",
        "cnh" => "cnh",
        "pis" | "pasep" | "nis" => "pis",
        "renavam" => "renavam",
        "voter" | "voter-id" | "titulo" | "titulo-eleitoral" => "voter-id",
        "cep" => "cep",
        "phone" | "phone-br" | "telefone" => "phone-br",
        "plate" | "placa" => "plate",
        "cns" => "cns",
        "rg" => "rg",
        "ie" | "inscricao-estadual" => "ie",
        "pix" => "pix",
        "person" | "gen-person" => "person",
        _ => "auto",
    }
}

fn normalize_uf(uf: &str) -> Option<String> {
    let value = uf.trim().to_ascii_uppercase();
    if value.len() == 2 && UF_CODES.contains(&value.as_str()) {
        Some(value)
    } else {
        None
    }
}

fn validate_cpf(input: &str) -> bool {
    let d = only_digits(input);
    if d.len() != 11 || all_equal(&d) {
        return false;
    }
    let digits = digits(&d);
    let dv1 = cpf_digit(&digits[0..9], &[10, 9, 8, 7, 6, 5, 4, 3, 2]);
    let dv2 = cpf_digit(&digits[0..10], &[11, 10, 9, 8, 7, 6, 5, 4, 3, 2]);
    dv1 == digits[9] && dv2 == digits[10]
}

fn cpf_digit(values: &[i32], weights: &[i32]) -> i32 {
    let rest = weighted_sum(values, weights) * 10 % 11;
    if rest == 10 { 0 } else { rest }
}

fn format_cpf(input: &str) -> Result<String, ()> {
    let d = only_digits(input);
    if d.len() != 11 {
        return Err(());
    }
    Ok(format!("{}.{}.{}-{}", &d[0..3], &d[3..6], &d[6..9], &d[9..11]))
}

fn origin_cpf(input: &str) -> Result<String, ()> {
    let d = only_digits(input);
    if d.len() < 9 {
        return Err(());
    }
    let key = (d.as_bytes()[8] - b'0') as i32;
    CPF_REGIONS.iter().find(|(k, _)| *k == key).map(|(_, region)| region.to_string()).ok_or(())
}

fn generate_cpf(rng: &mut BrRng<'_>) -> Result<String, JsValue> {
    let mut n = Vec::with_capacity(11);
    for _ in 0..9 {
        n.push(rng.digit()?);
    }
    n.push(cpf_digit(&n, &[10, 9, 8, 7, 6, 5, 4, 3, 2]));
    n.push(cpf_digit(&n, &[11, 10, 9, 8, 7, 6, 5, 4, 3, 2]));
    Ok(digits_to_string(&n))
}

fn generate_cpf_for_uf(uf: &str, rng: &mut BrRng<'_>) -> Result<String, JsValue> {
    let region = cpf_region_for_uf(uf).unwrap_or(8);
    let mut n = Vec::with_capacity(11);
    for _ in 0..8 {
        n.push(rng.digit()?);
    }
    n.push(region);
    n.push(cpf_digit(&n, &[10, 9, 8, 7, 6, 5, 4, 3, 2]));
    n.push(cpf_digit(&n, &[11, 10, 9, 8, 7, 6, 5, 4, 3, 2]));
    Ok(digits_to_string(&n))
}

fn validate_cnpj(input: &str) -> bool {
    let c = clean_alnum(input);
    if c.len() != 14 || all_equal(&c) {
        return false;
    }
    let bytes = c.as_bytes();
    if !bytes[12].is_ascii_digit() || !bytes[13].is_ascii_digit() {
        return false;
    }
    let base = &c[0..12];
    let dv1 = cnpj_digit(base);
    let dv2 = cnpj_digit(&format!("{base}{dv1}"));
    dv1 == (bytes[12] - b'0') as i32 && dv2 == (bytes[13] - b'0') as i32
}

fn cnpj_digit(base: &str) -> i32 {
    let weights = [2, 3, 4, 5, 6, 7, 8, 9];
    let mut sum = 0;
    let mut weight = 0;
    for b in base.bytes().rev() {
        sum += char_value(b) * weights[weight];
        weight = (weight + 1) % weights.len();
    }
    let rest = sum % 11;
    if rest == 0 || rest == 1 { 0 } else { 11 - rest }
}

fn format_cnpj(input: &str) -> Result<String, ()> {
    let c = clean_alnum(input);
    if c.len() != 14 {
        return Err(());
    }
    Ok(format!("{}.{}.{}/{}-{}", &c[0..2], &c[2..5], &c[5..8], &c[8..12], &c[12..14]))
}

fn generate_cnpj(rng: &mut BrRng<'_>, legacy: bool) -> Result<String, JsValue> {
    let alphabet = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut base = String::with_capacity(12);
    for _ in 0..12 {
        if legacy {
            base.push(char::from(b'0' + rng.digit()? as u8));
        } else {
            base.push(char::from(alphabet[rng.index(alphabet.len())?]));
        }
    }
    let dv1 = cnpj_digit(&base);
    let dv2 = cnpj_digit(&format!("{base}{dv1}"));
    Ok(format!("{base}{dv1}{dv2}"))
}

fn validate_cnh(input: &str) -> bool {
    let d = only_digits(input);
    if d.len() != 11 || all_equal(&d) {
        return false;
    }
    let (a, b) = cnh_digits(&d[0..9]);
    let bytes = d.as_bytes();
    a == (bytes[9] - b'0') as i32 && b == (bytes[10] - b'0') as i32
}

fn cnh_digits(base: &str) -> (i32, i32) {
    let bytes = base.as_bytes();
    let mut total = 0;
    for (i, b) in bytes.iter().enumerate().take(9) {
        total += (*b - b'0') as i32 * (9 - i as i32);
    }
    let r = total % 11;
    let (dv1, dsc) = if r >= 10 { (0, 2) } else { (r, 0) };
    total = 0;
    for (i, b) in bytes.iter().enumerate().take(9) {
        total += (*b - b'0') as i32 * (1 + i as i32);
    }
    let mut r = total % 11 - dsc;
    if r < 0 {
        r += 11;
    }
    (dv1, if r >= 10 { 0 } else { r })
}

fn generate_cnh(rng: &mut BrRng<'_>) -> Result<String, JsValue> {
    let mut base = String::with_capacity(9);
    for _ in 0..9 {
        base.push(char::from(b'0' + rng.digit()? as u8));
    }
    let (a, b) = cnh_digits(&base);
    Ok(format!("{base}{a}{b}"))
}

fn validate_pis(input: &str) -> bool {
    let d = only_digits(input);
    if d.len() != 11 || all_equal(&d) {
        return false;
    }
    let digits = digits(&d);
    pis_digit(&digits[0..10]) == digits[10]
}

fn pis_digit(values: &[i32]) -> i32 {
    let rest = weighted_sum(values, &[3, 2, 9, 8, 7, 6, 5, 4, 3, 2]) % 11;
    if rest == 0 || rest == 1 { 0 } else { 11 - rest }
}

fn format_pis(input: &str) -> Result<String, ()> {
    let d = only_digits(input);
    if d.len() != 11 {
        return Err(());
    }
    Ok(format!("{}.{}.{}-{}", &d[0..3], &d[3..8], &d[8..10], &d[10..11]))
}

fn generate_pis(rng: &mut BrRng<'_>) -> Result<String, JsValue> {
    let mut n = Vec::with_capacity(11);
    for _ in 0..10 {
        n.push(rng.digit()?);
    }
    n.push(pis_digit(&n));
    Ok(digits_to_string(&n))
}

fn validate_renavam(input: &str) -> bool {
    let d = only_digits(input);
    if d.len() != 11 || all_equal(&d) {
        return false;
    }
    let digits = digits(&d);
    renavam_digit(&digits[0..10]) == digits[10]
}

fn renavam_digit(values: &[i32]) -> i32 {
    let rest = weighted_sum(values, &[3, 2, 9, 8, 7, 6, 5, 4, 3, 2]) * 10 % 11;
    if rest == 10 { 0 } else { rest }
}

fn format_renavam(input: &str) -> Result<String, ()> {
    let d = only_digits(input);
    if d.len() > 11 || d.is_empty() {
        return Err(());
    }
    Ok(format!("{}{}", "0".repeat(11 - d.len()), d))
}

fn generate_renavam(rng: &mut BrRng<'_>) -> Result<String, JsValue> {
    let mut n = Vec::with_capacity(11);
    for _ in 0..10 {
        n.push(rng.digit()?);
    }
    n.push(renavam_digit(&n));
    Ok(digits_to_string(&n))
}

fn validate_voter_id(input: &str) -> bool {
    let d = only_digits(input);
    if d.len() != 12 || all_equal(&d) {
        return false;
    }
    let code = d[8..10].parse::<i32>().unwrap_or(0);
    if !(1..=28).contains(&code) {
        return false;
    }
    let v1 = voter_dv1(&d);
    let v2 = voter_dv2(&d, v1);
    let b = d.as_bytes();
    v1 == (b[10] - b'0') as i32 && v2 == (b[11] - b'0') as i32
}

fn voter_dv1(d: &str) -> i32 {
    let values = digits(&d[0..8]);
    let rest = weighted_sum(&values, &[2, 3, 4, 5, 6, 7, 8, 9]) % 11;
    if rest >= 10 { 0 } else { rest }
}

fn voter_dv2(d: &str, dv1: i32) -> i32 {
    let b = d.as_bytes();
    let values = [(b[8] - b'0') as i32, (b[9] - b'0') as i32, dv1];
    let rest = weighted_sum(&values, &[7, 8, 9]) % 11;
    if rest >= 10 { 0 } else { rest }
}

fn format_voter_id(input: &str) -> Result<String, ()> {
    let d = only_digits(input);
    if d.len() != 12 {
        return Err(());
    }
    Ok(format!("{} {} {}", &d[0..4], &d[4..8], &d[8..12]))
}

fn origin_voter_id(input: &str) -> Result<String, ()> {
    let d = only_digits(input);
    if d.len() != 12 {
        return Err(());
    }
    let code = d[8..10].parse::<i32>().map_err(|_| ())?;
    VOTER_UF_NAMES.iter().find(|(k, _)| *k == code).map(|(_, name)| name.to_string()).ok_or(())
}

fn generate_voter_id(uf: Option<&str>, rng: &mut BrRng<'_>) -> Result<String, JsValue> {
    let uf_code = uf.and_then(voter_code_for_uf).unwrap_or_else(|| 1 + rng.index(27).unwrap_or(0) as i32);
    let mut d = vec![0; 12];
    for item in d.iter_mut().take(8) {
        *item = rng.digit()?;
    }
    d[8] = uf_code / 10;
    d[9] = uf_code % 10;
    let s = digits_to_string(&d[0..10]);
    d[10] = voter_dv1(&s);
    d[11] = voter_dv2(&s, d[10]);
    Ok(digits_to_string(&d))
}

fn validate_cep(input: &str) -> bool {
    let d = only_digits(input);
    d.len() == 8 && origin_cep(input).is_ok()
}

fn format_cep(input: &str) -> Result<String, ()> {
    let d = only_digits(input);
    if d.len() != 8 {
        return Err(());
    }
    Ok(format!("{}-{}", &d[0..5], &d[5..8]))
}

fn origin_cep(input: &str) -> Result<String, ()> {
    let d = only_digits(input);
    if d.len() != 8 {
        return Err(());
    }
    let prefix = d[0..3].parse::<i32>().map_err(|_| ())?;
    CEP_RANGES.iter().find(|r| r.from <= prefix && prefix <= r.to).map(|r| r.uf.to_string()).ok_or(())
}

fn generate_cep(uf: Option<&str>, rng: &mut BrRng<'_>) -> Result<String, JsValue> {
    let ranges: Vec<&CepRange> = CEP_RANGES
        .iter()
        .filter(|range| uf.map_or(true, |state| range.uf == state))
        .collect();
    let range = ranges[rng.index(ranges.len())?];
    let prefix = range.from + rng.index((range.to - range.from + 1) as usize)? as i32;
    let suffix = rng.index(100000)?;
    Ok(format!("{prefix:03}{suffix:05}"))
}

fn national_phone_digits(digits: &str) -> Option<String> {
    let n = if digits.starts_with("0055") {
        &digits[4..]
    } else if digits.starts_with("55") && digits.len() > 11 {
        &digits[2..]
    } else {
        digits
    };
    if n.len() != 10 && n.len() != 11 {
        return None;
    }
    if ddd_uf(&n[0..2]).is_none() {
        return None;
    }
    if n.len() == 11 && n.as_bytes()[2] != b'9' {
        return None;
    }
    Some(n.to_string())
}

fn mask_phone_national(n: &str) -> String {
    if n.len() == 11 {
        format!("({}) {}-{}", &n[0..2], &n[2..7], &n[7..11])
    } else {
        format!("({}) {}-{}", &n[0..2], &n[2..6], &n[6..10])
    }
}

fn generate_phone(uf: Option<&str>, rng: &mut BrRng<'_>) -> Result<String, JsValue> {
    let ddds: Vec<&str> = DDD_TO_UF
        .iter()
        .filter(|(_, state)| uf.map_or(true, |want| *state == want))
        .map(|(ddd, _)| *ddd)
        .collect();
    let ddd = ddds[rng.index(ddds.len())?];
    let mut out = format!("{ddd}9");
    for _ in 0..8 {
        out.push(char::from(b'0' + rng.digit()? as u8));
    }
    Ok(out)
}

fn validate_plate(input: &str) -> bool {
    format_plate(input).is_ok()
}

fn format_plate(input: &str) -> Result<String, ()> {
    let v: String = input.chars().filter(|c| c.is_ascii_alphanumeric()).map(|c| c.to_ascii_uppercase()).collect();
    let b = v.as_bytes();
    if b.len() != 7 {
        return Err(());
    }
    let national = b[0..3].iter().all(|c| c.is_ascii_uppercase()) && b[3..7].iter().all(|c| c.is_ascii_digit());
    let mercosul = b[0..3].iter().all(|c| c.is_ascii_uppercase())
        && b[3].is_ascii_digit()
        && b[4].is_ascii_uppercase()
        && b[5].is_ascii_digit()
        && b[6].is_ascii_digit();
    if mercosul {
        Ok(v)
    } else if national {
        Ok(format!("{}-{}", &v[0..3], &v[3..7]))
    } else {
        Err(())
    }
}

fn generate_plate(rng: &mut BrRng<'_>) -> Result<String, JsValue> {
    let letters = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut out = String::new();
    for _ in 0..3 {
        out.push(char::from(letters[rng.index(letters.len())?]));
    }
    if rng.index(2)? == 0 {
        out.push('-');
        for _ in 0..4 {
            out.push(char::from(b'0' + rng.digit()? as u8));
        }
    } else {
        out.push(char::from(b'0' + rng.digit()? as u8));
        out.push(char::from(letters[rng.index(letters.len())?]));
        out.push(char::from(b'0' + rng.digit()? as u8));
        out.push(char::from(b'0' + rng.digit()? as u8));
    }
    Ok(out)
}

fn validate_cns(input: &str) -> bool {
    let d = only_digits(input);
    if d.len() != 15 || all_equal(&d) || !matches!(d.as_bytes()[0], b'1' | b'2' | b'7' | b'8' | b'9') {
        return false;
    }
    let values = digits(&d);
    weighted_sum(&values, &[15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1]) % 11 == 0
}

fn generate_cns(rng: &mut BrRng<'_>) -> Result<String, JsValue> {
    let prefixes = [1, 2, 7, 8, 9];
    loop {
        let mut d = Vec::with_capacity(15);
        d.push(prefixes[rng.index(prefixes.len())?]);
        for _ in 1..14 {
            d.push(rng.digit()?);
        }
        let partial = weighted_sum(&d, &[15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2]);
        let last = (11 - (partial % 11)) % 11;
        if last != 10 {
            d.push(last);
            return Ok(digits_to_string(&d));
        }
    }
}

fn validate_rg(input: &str, uf: &str) -> bool {
    if uf != "SP" {
        return false;
    }
    let Some((base, check)) = parse_rg(input) else {
        return false;
    };
    rg_digit(&base) == check
}

fn parse_rg(input: &str) -> Option<([i32; 8], i32)> {
    let cleaned: Vec<u8> = input.bytes().filter(|b| b.is_ascii_digit() || matches!(*b, b'X' | b'x')).collect();
    if cleaned.len() != 9 {
        return None;
    }
    let mut base = [0; 8];
    for (i, slot) in base.iter_mut().enumerate() {
        if !cleaned[i].is_ascii_digit() {
            return None;
        }
        *slot = (cleaned[i] - b'0') as i32;
    }
    let check = match cleaned[8] {
        b'X' | b'x' => 10,
        b'0' => 11,
        b'1'..=b'9' => (cleaned[8] - b'0') as i32,
        _ => return None,
    };
    Some((base, check))
}

fn rg_digit(base: &[i32; 8]) -> i32 {
    11 - weighted_sum(base, &[2, 3, 4, 5, 6, 7, 8, 9]) % 11
}

fn format_rg(input: &str) -> Result<String, ()> {
    let (base, check) = parse_rg(input).ok_or(())?;
    let d = digits_to_string(&base);
    let check = match check {
        10 => "X".to_string(),
        11 => "0".to_string(),
        n => n.to_string(),
    };
    Ok(format!("{}.{}.{}-{}", &d[0..2], &d[2..5], &d[5..8], check))
}

fn generate_rg(rng: &mut BrRng<'_>) -> Result<String, JsValue> {
    let mut base = [0; 8];
    for item in &mut base {
        *item = rng.digit()?;
    }
    let d = digits_to_string(&base);
    let check = match rg_digit(&base) {
        10 => "X".to_string(),
        11 => "0".to_string(),
        n => n.to_string(),
    };
    Ok(format!("{}.{}.{}-{}", &d[0..2], &d[2..5], &d[5..8], check))
}

fn validate_ie_for_uf(input: &str, uf: &str) -> bool {
    let d = only_digits(input);
    match uf {
        "SP" => d.len() == 12 && ie_sp_valid(&d),
        "MG" => d.len() == 13 && ie_mg_valid(&d),
        "RS" => d.len() == 10 && ie_rs_valid(&d),
        "PR" => d.len() == 10 && ie_pr_valid(&d),
        _ => false,
    }
}

fn format_ie_for_uf(input: &str, uf: &str) -> Result<String, ()> {
    let d = only_digits(input);
    if !validate_ie_for_uf(&d, uf) {
        return Err(());
    }
    match uf {
        "SP" => Ok(format!("{}.{}.{}.{}", &d[0..3], &d[3..6], &d[6..9], &d[9..12])),
        "MG" => Ok(format!("{}.{}.{}/{}", &d[0..3], &d[3..6], &d[6..9], &d[9..13])),
        "RS" => Ok(format!("{}/{}", &d[0..3], &d[3..10])),
        "PR" => Ok(format!("{}.{}-{}", &d[0..3], &d[3..8], &d[8..10])),
        _ => Err(()),
    }
}

fn generate_ie(uf: &str, rng: &mut BrRng<'_>) -> Result<String, JsValue> {
    match uf {
        "MG" => generate_ie_mg(rng),
        "RS" => generate_ie_rs(rng),
        "PR" => generate_ie_pr(rng),
        _ => generate_ie_sp(rng),
    }
}

fn ie_rightmost(input: &str, weights: &[i32]) -> i32 {
    weighted_sum(&digits(input), weights) % 11 % 10
}

fn ie_mod11(sum: i32) -> i32 {
    let dv = 11 - sum % 11;
    if dv >= 10 { 0 } else { dv }
}

fn ie_sp_valid(d: &str) -> bool {
    let b = d.as_bytes();
    ie_rightmost(&d[0..8], &[1, 3, 4, 5, 6, 7, 8, 10]) == (b[8] - b'0') as i32
        && ie_rightmost(&d[0..11], &[3, 2, 10, 9, 8, 7, 6, 5, 4, 3, 2]) == (b[11] - b'0') as i32
}

fn generate_ie_sp(rng: &mut BrRng<'_>) -> Result<String, JsValue> {
    let mut d = vec![0; 12];
    for item in d.iter_mut().take(8) {
        *item = rng.digit()?;
    }
    let s = digits_to_string(&d);
    d[8] = ie_rightmost(&s[0..8], &[1, 3, 4, 5, 6, 7, 8, 10]);
    d[9] = rng.digit()?;
    d[10] = rng.digit()?;
    let s = digits_to_string(&d);
    d[11] = ie_rightmost(&s[0..11], &[3, 2, 10, 9, 8, 7, 6, 5, 4, 3, 2]);
    format_ie_for_uf(&digits_to_string(&d), "SP").map_err(|_| err("IE-SP generation failed"))
}

fn ie_mg_digits(base11: &str) -> (i32, i32) {
    let base12 = format!("{}0{}", &base11[0..3], &base11[3..11]);
    let mut total = 0;
    for (b, w) in base12.bytes().zip([1, 2, 1, 2, 1, 2, 1, 2, 1, 2, 1, 2]) {
        let p = (b - b'0') as i32 * w;
        total += p / 10 + p % 10;
    }
    let d1 = (10 - total % 10) % 10;
    let base = format!("{base11}{d1}");
    let d2 = ie_mod11(weighted_sum(&digits(&base), &[3, 2, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2]));
    (d1, d2)
}

fn ie_mg_valid(d: &str) -> bool {
    let (a, b) = ie_mg_digits(&d[0..11]);
    let bytes = d.as_bytes();
    a == (bytes[11] - b'0') as i32 && b == (bytes[12] - b'0') as i32
}

fn generate_ie_mg(rng: &mut BrRng<'_>) -> Result<String, JsValue> {
    let mut base = String::with_capacity(11);
    for _ in 0..11 {
        base.push(char::from(b'0' + rng.digit()? as u8));
    }
    let (a, b) = ie_mg_digits(&base);
    format_ie_for_uf(&format!("{base}{a}{b}"), "MG").map_err(|_| err("IE-MG generation failed"))
}

fn ie_rs_valid(d: &str) -> bool {
    let b = d.as_bytes();
    ie_mod11(weighted_sum(&digits(&d[0..9]), &[2, 9, 8, 7, 6, 5, 4, 3, 2])) == (b[9] - b'0') as i32
}

fn generate_ie_rs(rng: &mut BrRng<'_>) -> Result<String, JsValue> {
    let mut base = String::with_capacity(9);
    for _ in 0..9 {
        base.push(char::from(b'0' + rng.digit()? as u8));
    }
    let dv = ie_mod11(weighted_sum(&digits(&base), &[2, 9, 8, 7, 6, 5, 4, 3, 2]));
    format_ie_for_uf(&format!("{base}{dv}"), "RS").map_err(|_| err("IE-RS generation failed"))
}

fn ie_pr_digits(base8: &str) -> (i32, i32) {
    let vals = digits(base8);
    let d1 = ie_mod11(weighted_sum(&vals, &[3, 2, 7, 6, 5, 4, 3, 2]));
    let d2 = ie_mod11(weighted_sum(&vals, &[4, 3, 2, 7, 6, 5, 4, 3]) + 2 * d1);
    (d1, d2)
}

fn ie_pr_valid(d: &str) -> bool {
    let (a, b) = ie_pr_digits(&d[0..8]);
    let bytes = d.as_bytes();
    a == (bytes[8] - b'0') as i32 && b == (bytes[9] - b'0') as i32
}

fn generate_ie_pr(rng: &mut BrRng<'_>) -> Result<String, JsValue> {
    let mut base = String::with_capacity(8);
    for _ in 0..8 {
        base.push(char::from(b'0' + rng.digit()? as u8));
    }
    let (a, b) = ie_pr_digits(&base);
    format_ie_for_uf(&format!("{base}{a}{b}"), "PR").map_err(|_| err("IE-PR generation failed"))
}

fn detect_pix_kind(input: &str) -> Option<&'static str> {
    let value = input.trim();
    if is_uuid_v4(value) {
        return Some("evp");
    }
    if value.contains('@') {
        return if is_email(value) { Some("email") } else { None };
    }
    if value.starts_with('+') {
        return if is_pix_phone(value) { Some("phone") } else { None };
    }
    let d_len = only_digits(value).len();
    if d_len == 11 && validate_cpf(value) {
        return Some("cpf");
    }
    if d_len == 14 && validate_cnpj(value) {
        return Some("cnpj");
    }
    None
}

fn generate_pix(rng: &mut BrRng<'_>) -> Result<String, JsValue> {
    let mut bytes = [0u8; 16];
    for byte in &mut bytes {
        *byte = rng.byte()?;
    }
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Ok(uuid_string(&bytes))
}

fn is_uuid_v4(value: &str) -> bool {
    let b = value.as_bytes();
    b.len() == 36
        && b[8] == b'-'
        && b[13] == b'-'
        && b[18] == b'-'
        && b[23] == b'-'
        && b[14] == b'4'
        && matches!(b[19], b'8' | b'9' | b'a' | b'b' | b'A' | b'B')
        && b.iter().enumerate().all(|(i, c)| matches!(i, 8 | 13 | 18 | 23) || c.is_ascii_hexdigit())
}

fn is_pix_phone(value: &str) -> bool {
    let b = value.as_bytes();
    (b.len() == 13 || b.len() == 14)
        && b.starts_with(b"+55")
        && b[3..].iter().all(|c| c.is_ascii_digit())
        && phone_summary(value).is_some()
}

fn is_email(value: &str) -> bool {
    let Some(at) = value.find('@') else {
        return false;
    };
    if value[at + 1..].contains('@') {
        return false;
    }
    let local = &value[..at];
    let domain = &value[at + 1..];
    if local.is_empty() || domain.is_empty() {
        return false;
    }
    if !local.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'_' | b'%' | b'+' | b'-')) {
        return false;
    }
    let labels: Vec<&str> = domain.split('.').collect();
    labels.len() >= 2 && labels.iter().all(|label| {
        let b = label.as_bytes();
        !b.is_empty()
            && b[0].is_ascii_alphanumeric()
            && b[b.len() - 1].is_ascii_alphanumeric()
            && b.iter().all(|c| c.is_ascii_alphanumeric() || *c == b'-')
    })
}

fn only_digits(input: &str) -> String {
    input.chars().filter(|c| c.is_ascii_digit()).collect()
}

fn clean_alnum(input: &str) -> String {
    input
        .bytes()
        .filter_map(|b| {
            let up = b.to_ascii_uppercase();
            if up.is_ascii_digit() || up.is_ascii_uppercase() {
                Some(up as char)
            } else {
                None
            }
        })
        .collect()
}

fn all_equal(input: &str) -> bool {
    let b = input.as_bytes();
    !b.is_empty() && b.iter().all(|byte| *byte == b[0])
}

fn digits(input: &str) -> Vec<i32> {
    input.bytes().map(|b| (b - b'0') as i32).collect()
}

fn digits_to_string<T: AsRef<[i32]>>(values: T) -> String {
    values.as_ref().iter().map(|n| char::from(b'0' + *n as u8)).collect()
}

fn char_value(b: u8) -> i32 {
    if b.is_ascii_digit() {
        (b - b'0') as i32
    } else if b.is_ascii_uppercase() {
        (b - b'A') as i32 + 17
    } else {
        -1
    }
}

fn weighted_sum(values: &[i32], weights: &[i32]) -> i32 {
    values.iter().zip(weights.iter()).map(|(value, weight)| value * weight).sum()
}

fn q(value: &str) -> String {
    format!("\"{}\"", json_escape(value))
}

fn json_escape(value: &str) -> String {
    value
        .chars()
        .flat_map(|c| match c {
            '"' => "\\\"".chars().collect::<Vec<_>>(),
            '\\' => "\\\\".chars().collect(),
            '\n' => "\\n".chars().collect(),
            '\r' => "\\r".chars().collect(),
            '\t' => "\\t".chars().collect(),
            _ => vec![c],
        })
        .collect()
}

fn err(message: &str) -> JsValue {
    JsValue::from_str(message)
}

fn uuid_string(bytes: &[u8; 16]) -> String {
    let hex = "0123456789abcdef".as_bytes();
    let mut out = String::with_capacity(36);
    for (idx, byte) in bytes.iter().enumerate() {
        if matches!(idx, 4 | 6 | 8 | 10) {
            out.push('-');
        }
        out.push(char::from(hex[(byte >> 4) as usize]));
        out.push(char::from(hex[(byte & 0xf) as usize]));
    }
    out
}

struct BrRng<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> BrRng<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    fn byte(&mut self) -> Result<u8, JsValue> {
        let Some(value) = self.bytes.get(self.pos) else {
            return Err(err("not enough random bytes"));
        };
        self.pos += 1;
        Ok(*value)
    }

    fn u32(&mut self) -> Result<u32, JsValue> {
        Ok(u32::from_le_bytes([self.byte()?, self.byte()?, self.byte()?, self.byte()?]))
    }

    fn index(&mut self, max: usize) -> Result<usize, JsValue> {
        if max == 0 {
            return Err(err("empty random range"));
        }
        let limit = u32::MAX - (u32::MAX % max as u32);
        loop {
            let value = self.u32()?;
            if value < limit {
                return Ok((value % max as u32) as usize);
            }
        }
    }

    fn digit(&mut self) -> Result<i32, JsValue> {
        Ok(self.index(10)? as i32)
    }
}

fn ddd_uf(ddd: &str) -> Option<&'static str> {
    DDD_TO_UF.iter().find(|(code, _)| *code == ddd).map(|(_, uf)| *uf)
}

fn cpf_region_for_uf(uf: &str) -> Option<i32> {
    match uf {
        "RS" => Some(0),
        "DF" | "GO" | "MS" | "MT" | "TO" => Some(1),
        "AC" | "AM" | "AP" | "PA" | "RO" | "RR" => Some(2),
        "CE" | "MA" | "PI" => Some(3),
        "AL" | "PB" | "PE" | "RN" => Some(4),
        "BA" | "SE" => Some(5),
        "MG" => Some(6),
        "ES" | "RJ" => Some(7),
        "SP" => Some(8),
        "PR" | "SC" => Some(9),
        _ => None,
    }
}

fn voter_code_for_uf(uf: &str) -> Option<i32> {
    match uf {
        "SP" => Some(1), "MG" => Some(2), "RJ" => Some(3), "RS" => Some(4), "BA" => Some(5),
        "PR" => Some(6), "CE" => Some(7), "PE" => Some(8), "SC" => Some(9), "GO" => Some(10),
        "MA" => Some(11), "PB" => Some(12), "PA" => Some(13), "ES" => Some(14), "PI" => Some(15),
        "RN" => Some(16), "AL" => Some(17), "MT" => Some(18), "MS" => Some(19), "DF" => Some(20),
        "SE" => Some(21), "AM" => Some(22), "RO" => Some(23), "AC" => Some(24), "AP" => Some(25),
        "RR" => Some(26), "TO" => Some(27),
        _ => None,
    }
}

const UF_CODES: &[&str] = &[
    "AC", "AL", "AP", "AM", "BA", "CE", "DF", "ES", "GO", "MA", "MT", "MS", "MG",
    "PA", "PB", "PR", "PE", "PI", "RJ", "RN", "RS", "RO", "RR", "SC", "SP", "SE",
    "TO",
];

const FIRST_NAMES: &[&str] = &["Ana", "Bruno", "Carla", "Diego", "Eva", "Felipe", "Julia", "Lucas"];
const LAST_NAMES: &[&str] = &["Silva", "Santos", "Oliveira", "Souza", "Costa", "Pereira", "Lima", "Moura"];
const COMPANY_SUFFIXES: &[&str] = &["Tecnologia Ltda", "Servicos Digitais", "Comercio Brasil", "Sistemas SA"];

const CPF_REGIONS: &[(i32, &str)] = &[
    (0, "Rio Grande do Sul"),
    (1, "Federal District, Goias, Mato Grosso do Sul, Mato Grosso and Tocantins"),
    (2, "Para, Amazonas, Acre, Amapa, Rondonia and Roraima"),
    (3, "Ceara, Maranhao and Piaui"),
    (4, "Pernambuco, Rio Grande do Norte, Paraiba and Alagoas"),
    (5, "Bahia and Sergipe"),
    (6, "Minas Gerais"),
    (7, "Rio de Janeiro and Espirito Santo"),
    (8, "Sao Paulo"),
    (9, "Parana and Santa Catarina"),
];

const VOTER_UF_NAMES: &[(i32, &str)] = &[
    (1, "Sao Paulo"), (2, "Minas Gerais"), (3, "Rio de Janeiro"), (4, "Rio Grande do Sul"),
    (5, "Bahia"), (6, "Parana"), (7, "Ceara"), (8, "Pernambuco"), (9, "Santa Catarina"),
    (10, "Goias"), (11, "Maranhao"), (12, "Paraiba"), (13, "Para"), (14, "Espirito Santo"),
    (15, "Piaui"), (16, "Rio Grande do Norte"), (17, "Alagoas"), (18, "Mato Grosso"),
    (19, "Mato Grosso do Sul"), (20, "Distrito Federal"), (21, "Sergipe"), (22, "Amazonas"),
    (23, "Rondonia"), (24, "Acre"), (25, "Amapa"), (26, "Roraima"), (27, "Tocantins"),
    (28, "Exterior"),
];

const CEP_RANGES: &[CepRange] = &[
    CepRange { uf: "SP", from: 10, to: 199 }, CepRange { uf: "RJ", from: 200, to: 289 },
    CepRange { uf: "ES", from: 290, to: 299 }, CepRange { uf: "MG", from: 300, to: 399 },
    CepRange { uf: "BA", from: 400, to: 489 }, CepRange { uf: "SE", from: 490, to: 499 },
    CepRange { uf: "PE", from: 500, to: 569 }, CepRange { uf: "AL", from: 570, to: 579 },
    CepRange { uf: "PB", from: 580, to: 589 }, CepRange { uf: "RN", from: 590, to: 599 },
    CepRange { uf: "CE", from: 600, to: 639 }, CepRange { uf: "PI", from: 640, to: 649 },
    CepRange { uf: "MA", from: 650, to: 659 }, CepRange { uf: "PA", from: 660, to: 688 },
    CepRange { uf: "AP", from: 689, to: 689 }, CepRange { uf: "AM", from: 690, to: 692 },
    CepRange { uf: "RR", from: 693, to: 693 }, CepRange { uf: "AM", from: 694, to: 698 },
    CepRange { uf: "AC", from: 699, to: 699 }, CepRange { uf: "DF", from: 700, to: 727 },
    CepRange { uf: "GO", from: 728, to: 729 }, CepRange { uf: "DF", from: 730, to: 736 },
    CepRange { uf: "GO", from: 737, to: 767 }, CepRange { uf: "RO", from: 768, to: 769 },
    CepRange { uf: "TO", from: 770, to: 779 }, CepRange { uf: "MT", from: 780, to: 788 },
    CepRange { uf: "MS", from: 790, to: 799 }, CepRange { uf: "PR", from: 800, to: 879 },
    CepRange { uf: "SC", from: 880, to: 899 }, CepRange { uf: "RS", from: 900, to: 999 },
];

const DDD_TO_UF: &[(&str, &str)] = &[
    ("11", "SP"), ("12", "SP"), ("13", "SP"), ("14", "SP"), ("15", "SP"), ("16", "SP"),
    ("17", "SP"), ("18", "SP"), ("19", "SP"), ("21", "RJ"), ("22", "RJ"), ("24", "RJ"),
    ("27", "ES"), ("28", "ES"), ("31", "MG"), ("32", "MG"), ("33", "MG"), ("34", "MG"),
    ("35", "MG"), ("37", "MG"), ("38", "MG"), ("41", "PR"), ("42", "PR"), ("43", "PR"),
    ("44", "PR"), ("45", "PR"), ("46", "PR"), ("47", "SC"), ("48", "SC"), ("49", "SC"),
    ("51", "RS"), ("53", "RS"), ("54", "RS"), ("55", "RS"), ("61", "DF"), ("62", "GO"),
    ("63", "TO"), ("64", "GO"), ("65", "MT"), ("66", "MT"), ("67", "MS"), ("68", "AC"),
    ("69", "RO"), ("71", "BA"), ("73", "BA"), ("74", "BA"), ("75", "BA"), ("77", "BA"),
    ("79", "SE"), ("81", "PE"), ("82", "AL"), ("83", "PB"), ("84", "RN"), ("85", "CE"),
    ("86", "PI"), ("87", "PE"), ("88", "CE"), ("89", "PI"), ("91", "PA"), ("92", "AM"),
    ("93", "PA"), ("94", "PA"), ("95", "RR"), ("96", "AP"), ("97", "AM"), ("98", "MA"),
    ("99", "MA"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_and_formats_core_documents() {
        let cpf = br_document("cpf", "validate", "529.982.247-25", "SP", &[]).unwrap();
        assert!(cpf.contains("\"valid\":true"));
        assert!(cpf.contains("\"formatted\":\"529.982.247-25\""));

        let cnpj = br_document("cnpj", "validate", "945HYB454H3560", "SP", &[]).unwrap();
        assert!(cnpj.contains("\"valid\":true"));

        let ie = br_document("ie", "validate", "110.042.490.114", "SP", &[]).unwrap();
        assert!(ie.contains("\"valid\":true"));
        assert!(ie.contains("\"origin\":\"SP\""));
    }

    #[test]
    fn summarizes_brazilian_phone_numbers() {
        let summary = phone_summary("+55 11930390628").unwrap();
        assert_eq!(summary.formatted, "(11) 93039-0628");
        assert_eq!(summary.e164, "+5511930390628");
        assert_eq!(summary.uf, "SP");
        assert!(phone_summary("+91 87308 44504").is_none());
        assert!(phone_summary("+1 1930390628").is_none());
    }

    #[test]
    fn generates_valid_synthetic_person() {
        let random: Vec<u8> = (0..4096).map(|index| (index % 251) as u8).collect();
        let json = br_document("person", "generate", "", "SP", &random).unwrap();
        assert!(json.contains("\"kind\":\"person\""));
        assert!(json.contains("\"valid\":true"));
        assert!(json.contains("\"cpf\""));
        assert!(json.contains("\"pix_keys\""));
    }
}
