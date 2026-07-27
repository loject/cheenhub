# JNI вызывает эти классы напрямую, поэтому оптимизатор не видит обычных ссылок из Kotlin.
-keep, includedescriptorclasses class org.rustls.platformverifier.** { *; }
