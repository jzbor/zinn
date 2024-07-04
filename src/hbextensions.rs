use std::process::Command;

use handlebars::{Context, Handlebars, Helper, HelperResult, JsonRender, Output, RenderContext, RenderErrorReason};


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

fn joinlines(h: &Helper, _: &Handlebars, _: &Context, _: &mut RenderContext, out: &mut dyn Output) -> HelperResult {
    if h.params().is_empty() {
        return Err(RenderErrorReason::ParamNotFoundForIndex("joinlines", h.params().len()).into())
    }

    let base = h.params().first().unwrap().value().render();
    out.write(&base.replace('\n', " "))?;

    Ok(())
}

pub fn register_helpers(handlebars: &mut Handlebars) {
      handlebars.register_helper("shell", Box::new(shell));
      handlebars.register_helper("subst", Box::new(subst));
      handlebars.register_helper("joinlines", Box::new(joinlines));

}
