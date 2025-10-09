use anyhow::Result;

slint::include_modules!();

fn main() -> Result<()> {
    let ui = AppWindow::new()?;
    let timer = std::rc::Rc::new(slint::Timer::default());
    let ui_weak = ui.as_weak();
    let timer_clone = timer.clone();

    let mut i = 0;
    slint::Timer::single_shot(std::time::Duration::from_millis(100), move || {
        timer_clone.start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_secs(1),
            move || {
                if let Some(win) = ui_weak.upgrade() {
                    i += 1;
                    win.invoke_log(format!("{i}").into());
                }
            },
        );
    });

    ui.run()?;

    Ok(())
}
