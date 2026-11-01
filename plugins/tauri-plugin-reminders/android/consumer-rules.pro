# JNI entry points are looked up by name from Rust symbols.
-keep class com.hisaab.reminders.Native { *; }
# WorkManager creates workers by class name.
-keep class com.hisaab.reminders.SyncWorker { <init>(...); }
# Room (inside WorkManager) instantiates WorkDatabase_Impl by reflection; R8 full
# mode stripped its no-arg constructor and every process start crashed (0.x, issue #6).
-keep class * extends androidx.room.RoomDatabase { <init>(); }
-keep class androidx.work.impl.WorkDatabase_Impl { *; }
