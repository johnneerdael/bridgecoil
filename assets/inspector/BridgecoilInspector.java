package nl.neerdael.bridgecoil;

import android.content.Context;
import android.content.Intent;
import android.content.pm.ApplicationInfo;
import android.content.pm.PackageInfo;
import android.content.pm.PackageManager;
import android.content.pm.ResolveInfo;
import android.content.pm.Signature;
import android.graphics.Bitmap;
import android.graphics.Canvas;
import android.graphics.drawable.Drawable;
import android.os.Build;
import android.os.Looper;
import android.os.UserHandle;
import android.util.Base64;
import org.json.JSONArray;
import org.json.JSONObject;
import java.io.ByteArrayOutputStream;
import java.lang.reflect.Constructor;
import java.security.MessageDigest;
import java.util.LinkedHashSet;

/** One-shot, read-only PackageManager inspection under the existing ADB shell UID. */
public final class BridgecoilInspector {
    public static void main(String[] args) {
        JSONObject response = new JSONObject();
        try {
            if (args.length < 2 || args.length > 33) throw new IllegalArgumentException("Expected profile and 1..32 packages");
            int user = Integer.parseInt(args[0]);
            if (user < 0) throw new IllegalArgumentException("Invalid profile");
            Looper.prepareMainLooper();
            Class<?> threadClass = Class.forName("android.app.ActivityThread");
            Object thread = threadClass.getMethod("systemMain").invoke(null);
            Context context = (Context) threadClass.getMethod("getSystemContext").invoke(thread);
            context = contextFor(context, user);
            PackageManager pm = context.getPackageManager();
            JSONArray items = new JSONArray();
            if (args.length == 2 && args[1].equals("--labels")) {
                int count = 0;
                for (ApplicationInfo app : pm.getInstalledApplications(PackageManager.MATCH_UNINSTALLED_PACKAGES)) {
                    if (count++ >= 1024) { response.put("partial", true); break; }
                    JSONObject item = new JSONObject().put("package", app.packageName);
                    try { item.put("label", bound(pm.getApplicationLabel(app).toString(), 96)); }
                    catch (Throwable ignored) { item.put("label", app.packageName); }
                    items.put(item);
                }
                response.put("items", items);
                System.out.println("__BRIDGECOIL_JSON__" + response.toString());
                System.exit(0);
                return;
            }
            for (int n = 1; n < args.length; n++) {
                String id = args[n];
                if (id.length() > 255 || !id.matches("[A-Za-z_][A-Za-z0-9_]*(\\.[A-Za-z_][A-Za-z0-9_]*)*")) throw new IllegalArgumentException("Invalid package");
                JSONObject item = new JSONObject().put("package", id);
                try {
                    int flags = PackageManager.GET_PERMISSIONS | PackageManager.MATCH_UNINSTALLED_PACKAGES;
                    flags |= Build.VERSION.SDK_INT >= 28 ? PackageManager.GET_SIGNING_CERTIFICATES : PackageManager.GET_SIGNATURES;
                    PackageInfo info = pm.getPackageInfo(id, flags);
                    ApplicationInfo app = info.applicationInfo;
                    item.put("label", bound(pm.getApplicationLabel(app).toString(), 256));
                    item.put("versionName", info.versionName == null ? JSONObject.NULL : bound(info.versionName, 256));
                    item.put("versionCode", Build.VERSION.SDK_INT >= 28 ? info.getLongVersionCode() : (long) info.versionCode);
                    item.put("system", (app.flags & ApplicationInfo.FLAG_SYSTEM) != 0);
                    item.put("enabled", app.enabled);
                    item.put("installed", (app.flags & ApplicationInfo.FLAG_INSTALLED) != 0);
                    item.put("targetSdk", app.targetSdkVersion);
                    try { item.put("icon", icon(app.loadIcon(pm))); } catch (Throwable ignored) { item.put("icon", JSONObject.NULL); }
                    LinkedHashSet<String> activities = new LinkedHashSet<>();
                    for (String category : new String[]{ Intent.CATEGORY_LEANBACK_LAUNCHER, Intent.CATEGORY_LAUNCHER, Intent.CATEGORY_HOME }) {
                        Intent intent = new Intent(Intent.ACTION_MAIN).addCategory(category).setPackage(id);
                        for (ResolveInfo resolve : pm.queryIntentActivities(intent, 0)) {
                            if (activities.size() >= 64) break;
                            if (resolve.activityInfo != null) activities.add(resolve.activityInfo.packageName + "/" + resolve.activityInfo.name);
                        }
                    }
                    item.put("activities", new JSONArray(activities));
                    JSONArray permissions = new JSONArray();
                    if (info.requestedPermissions != null) for (int p = 0; p < Math.min(info.requestedPermissions.length, 256); p++) {
                        String permission = info.requestedPermissions[p];
                        JSONObject entry = new JSONObject().put("name", bound(permission, 256));
                        boolean granted = info.requestedPermissionsFlags != null && p < info.requestedPermissionsFlags.length && (info.requestedPermissionsFlags[p] & PackageInfo.REQUESTED_PERMISSION_GRANTED) != 0;
                        entry.put("granted", granted);
                        try { entry.put("protection", pm.getPermissionInfo(permission, 0).protectionLevel & 15); } catch (Throwable ignored) { entry.put("protection", JSONObject.NULL); }
                        permissions.put(entry);
                    }
                    item.put("permissions", permissions);
                    JSONArray signers = new JSONArray();
                    Signature[] signatures = Build.VERSION.SDK_INT >= 28 && info.signingInfo != null ? info.signingInfo.getApkContentsSigners() : info.signatures;
                    if (signatures != null) for (int s = 0; s < Math.min(signatures.length, 16); s++) signers.put(sha256(signatures[s].toByteArray()));
                    item.put("signers", signers);
                } catch (Throwable e) { item.put("error", bound(e.getClass().getSimpleName() + ": " + e.getMessage(), 256)); }
                items.put(item);
            }
            response.put("items", items);
        } catch (Throwable e) {
            try { response.put("error", bound(e.getClass().getSimpleName() + ": " + e.getMessage(), 256)); } catch (Exception ignored) {}
        }
        System.out.println("__BRIDGECOIL_JSON__" + response.toString());
        System.exit(0);
    }
    private static Context contextFor(Context system, int user) throws Exception {
        Constructor<UserHandle> constructor = UserHandle.class.getDeclaredConstructor(int.class);
        constructor.setAccessible(true);
        UserHandle handle = constructor.newInstance(user);
        try {
            return (Context) Context.class.getMethod("createContextAsUser", UserHandle.class, int.class).invoke(system, handle, 0);
        } catch (NoSuchMethodException older) {
            // Older Android, including the Android 9 boxes people still use as TVs, only has this form.
            return (Context) Context.class.getMethod("createPackageContextAsUser", String.class, int.class, UserHandle.class).invoke(system, "android", 0, handle);
        }
    }
    private static String bound(String s, int limit) { return s.length() > limit ? s.substring(0, limit) : s; }
    private static String icon(Drawable drawable) throws Exception {
        Bitmap bitmap = Bitmap.createBitmap(64, 64, Bitmap.Config.ARGB_8888);
        drawable.setBounds(0, 0, 64, 64); drawable.draw(new Canvas(bitmap));
        ByteArrayOutputStream bytes = new ByteArrayOutputStream(); bitmap.compress(Bitmap.CompressFormat.PNG, 100, bytes); bitmap.recycle();
        if (bytes.size() > 32768) throw new IllegalArgumentException("Icon exceeds limit");
        return Base64.encodeToString(bytes.toByteArray(), Base64.NO_WRAP);
    }
    private static String sha256(byte[] data) throws Exception {
        byte[] digest = MessageDigest.getInstance("SHA-256").digest(data); StringBuilder result = new StringBuilder();
        for (byte b : digest) result.append(String.format(java.util.Locale.ROOT, "%02x", b & 255));
        return result.toString();
    }
}
