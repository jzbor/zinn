use std::process::Command;

use handlebars::{Context, Handlebars, Helper, HelperResult, JsonRender, Output, RenderContext, RenderErrorReason};


fn parse_list(s: &str) -> Vec<String> {
    s.split(char::is_whitespace)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_owned())
        .collect()
}

fn encode_list(l: &[String]) -> String {
    l.join(" ")
}


/// Concatenate all parameters
fn cat(h: &Helper, _: &Handlebars, _: &Context, _: &mut RenderContext, out: &mut dyn Output) -> HelperResult {
    let mut string = String::new();

    for param in h.params() {
        string.push_str(&param.value().render())
    }

    out.write(&string)?;
    Ok(())
}

/// Join lines and connect them with a regular whitespace
fn joinlines(h: &Helper, _: &Handlebars, _: &Context, _: &mut RenderContext, out: &mut dyn Output) -> HelperResult {
    if h.params().is_empty() {
        return Err(RenderErrorReason::ParamNotFoundForIndex("joinlines", h.params().len()).into())
    }

    let base = h.params().first().unwrap().value().render();
    out.write(&base.replace('\n', " "))?;

    Ok(())
}

/// Create a space-separated list from all input parameters
fn lst(h: &Helper, _: &Handlebars, _: &Context, _: &mut RenderContext, out: &mut dyn Output) -> HelperResult {
    if h.params().is_empty() {
        return Err(RenderErrorReason::ParamNotFoundForIndex("joinlines", h.params().len()).into())
    }

    let params: Vec<String> = h.params()
        .iter()
        .flat_map(|p| parse_list(&p.value().render()))
        .collect();

    out.write(&encode_list(&params))?;

    Ok(())
}

/// Add a prefix to each element in a space-separated list
fn lst_prefix(h: &Helper, _: &Handlebars, _: &Context, _: &mut RenderContext, out: &mut dyn Output) -> HelperResult {
    if h.params().len() < 2 {
        return Err(RenderErrorReason::ParamNotFoundForIndex("lst-prefix", h.params().len()).into())
    }

    let prefix = h.params().first().unwrap().value().render();
    let list = parse_list(&h.params().get(1).unwrap().value().render());

    let result: Vec<String> = list.iter()
        .map(|e| format!("{}{}", &prefix, e))
        .collect();

    out.write(&encode_list(&result))?;

    Ok(())
}

/// Apply a regex replacement operation to each element in a space-separated list
#[cfg(feature = "regex")]
fn lst_re(h: &Helper, _: &Handlebars, _: &Context, _: &mut RenderContext, out: &mut dyn Output) -> HelperResult {
    if h.params().len() < 3 {
        return Err(RenderErrorReason::ParamNotFoundForIndex("lst-re", h.params().len()).into())
    }

    let base = h.params().first().unwrap().value().render();
    let list = parse_list(&base);
    let pattern = h.params().get(1).unwrap().value().render();
    let replacement = h.params().get(2).unwrap().value().render();

    let re = regex_lite::Regex::new(&pattern)
        .map_err(|e| RenderErrorReason::Other(format!("regex error - {e}")))?;

    let result: Vec<String> = list.iter()
        .map(|e| re.replace_all(e, &replacement).to_string())
        .collect();

    out.write(&encode_list(&result))?;

    Ok(())
}

/// Add a suffix to each element in a space-separated list
fn lst_suffix(h: &Helper, _: &Handlebars, _: &Context, _: &mut RenderContext, out: &mut dyn Output) -> HelperResult {
    if h.params().len() < 2 {
        return Err(RenderErrorReason::ParamNotFoundForIndex("lst-suffix", h.params().len()).into())
    }

    let suffix = h.params().first().unwrap().value().render();
    let list = parse_list(&h.params().get(1).unwrap().value().render());

    let result: Vec<String> = list.iter()
        .map(|e| format!("{}{}", e, &suffix))
        .collect();

    out.write(&encode_list(&result))?;

    Ok(())
}

/// Create copy of a space-separated list without certain elements
fn lst_without(h: &Helper, _: &Handlebars, _: &Context, _: &mut RenderContext, out: &mut dyn Output) -> HelperResult {
    if h.params().len() < 2 {
        return Err(RenderErrorReason::ParamNotFoundForIndex("lst-without", h.params().len()).into())
    }

    let list = parse_list(&h.params().first().unwrap().value().render());
    let remove_list: Vec<String> = h.params().iter().skip(1).map(|p| p.value().render()).collect();

    let result: Vec<String> = list.into_iter()
        .filter(|e| !remove_list.contains(e))
        .collect();

    out.write(&encode_list(&result))?;

    Ok(())
}

/// Apply a regex replacement operation to an input string
#[cfg(feature = "regex")]
fn re(h: &Helper, _: &Handlebars, _: &Context, _: &mut RenderContext, out: &mut dyn Output) -> HelperResult {
    if h.params().len() < 3 {
        return Err(RenderErrorReason::ParamNotFoundForIndex("re", h.params().len()).into())
    }

    let base = h.params().first().unwrap().value().render();
    let pattern = h.params().get(1).unwrap().value().render();
    let replacement = h.params().get(2).unwrap().value().render();

    let re = regex_lite::Regex::new(&pattern)
        .map_err(|e| RenderErrorReason::Other(format!("regex error - {e}")))?;

    let result = re.replace_all(&base, &replacement);

    out.write(&result)?;

    Ok(())
}

/// Create a string from the output of a shell command
fn shell(h: &Helper, _: &Handlebars, _: &Context, _: &mut RenderContext, out: &mut dyn Output) -> HelperResult {
    if h.params().is_empty() {
        return Err(RenderErrorReason::ParamNotFoundForIndex("shell", h.params().len()).into())
    }

    let cmd = h.params().iter().fold(String::new(), |s, p| format!("{} {}", s, p.value().render()));
    let output = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output()?;

    if let Ok(string) = String::from_utf8(output.stdout) {
        if let Some(trimmed_string) = string.strip_suffix('\n') {
            out.write(trimmed_string)?;
        } else {
            out.write(&string)?;
        }
    }

    Ok(())
}

/// Replace all occurrences of a substring
fn subst(h: &Helper, _: &Handlebars, _: &Context, _: &mut RenderContext, out: &mut dyn Output) -> HelperResult {
    if h.params().len() < 3 {
        return Err(RenderErrorReason::ParamNotFoundForIndex("subst", h.params().len()).into())
    }

    let base = h.params().first().unwrap().value().render();
    let pattern = h.params().get(1).unwrap().value().render();
    let replacement = h.params().get(2).unwrap().value().render();
    out.write(&base.replace(&pattern, &replacement))?;

    Ok(())
}


pub fn register_helpers(handlebars: &mut Handlebars) {
    handlebars.register_helper("cat", Box::new(cat));
    handlebars.register_helper("joinlines", Box::new(joinlines));
    handlebars.register_helper("lst", Box::new(lst));
    handlebars.register_helper("lst-prefix", Box::new(lst_prefix));
    #[cfg(feature = "regex")]
    handlebars.register_helper("lst-re", Box::new(lst_re));
    handlebars.register_helper("lst-suffix", Box::new(lst_suffix));
    handlebars.register_helper("lst-without", Box::new(lst_without));
    #[cfg(feature = "regex")]
    handlebars.register_helper("re", Box::new(re));
    handlebars.register_helper("shell", Box::new(shell));
    handlebars.register_helper("subst", Box::new(subst));
}
