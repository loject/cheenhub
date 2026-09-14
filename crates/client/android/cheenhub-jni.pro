# Rust/JNI resolves these classes and methods by their exact JVM names.
# R8 may optimize their implementations, but must not rename or remove them.

-keep,allowoptimization class dev.dioxus.main.MainActivity {
    *;
}

-keep,allowoptimization class dev.dioxus.main.DioxusForegroundService {
    *;
}

-keep,allowoptimization class ru.cheenhub.imagepicker.** {
    *;
}
